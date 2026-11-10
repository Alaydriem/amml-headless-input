use std::sync::Arc;

use headless_input_core::run::RunDefaults;
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::{ControlFolder, Forwarder, InstanceId};
use crate::dto::{
    FeaturesDto, InstanceEntry, InstanceResult, InstanceTarget, LiveActionDto, MemberReply, MemberRequest, RunStarted,
};
use crate::port::{HostResolver, PortError};

// What the tools ask of the hub: resolve the instance they name, check a control file, forward the command to each
// target and gather the replies. A control file named by path is read from `folder`.
#[derive(Debug, Clone)]
pub struct HubCalls {
    forwarder: Arc<Forwarder>,
    folder: ControlFolder,
    resolver: Arc<HostResolver>,
}

impl HubCalls {
    pub fn new(forwarder: Arc<Forwarder>, folder: ControlFolder, resolver: Arc<HostResolver>) -> Self {
        Self { forwarder, folder, resolver }
    }

    // every connected instance with its state; an instance that does not answer shows `unknown`
    pub async fn instances(&self) -> Vec<InstanceEntry> {
        let members = self.forwarder.registry().list();
        let mut asks = JoinSet::new();
        for member in &members {
            let forwarder = Arc::clone(&self.forwarder);
            let id = member.id.clone();
            asks.spawn(async move {
                let state = match forwarder.send(&id, MemberRequest::Status).await {
                    Ok(MemberReply::Status(status)) => status.state,
                    _ => "unknown".to_owned(),
                };
                (id, state)
            });
        }
        let states = asks.join_all().await;
        members
            .into_iter()
            .map(|member| InstanceEntry {
                state: states
                    .iter()
                    .find(|(id, _)| *id == member.id)
                    .map_or_else(|| "unknown".to_owned(), |(_, state)| state.clone()),
                hub: member.local,
                instance: member.id.to_string(),
                pid: member.pid,
                features: FeaturesDto::from(member.features),
            })
            .collect()
    }

    // the request on the one instance named (or the only one connected)
    pub async fn one(&self, instance: Option<&str>, request: MemberRequest) -> Result<Value, PortError> {
        let ids = InstanceTarget::new(instance).resolve(&self.connected(), false)?;
        let mut replies = self.forward(ids, |_| Ok(request.clone())).await;
        replies.pop().map_or_else(|| Err(PortError::UnknownInstance(String::new())), |(_, reply)| reply)
    }

    // the request on the instance named, or with `"*"` on every instance with one result for each
    pub async fn each(&self, instance: Option<&str>, request: MemberRequest) -> Result<Value, PortError> {
        let target = InstanceTarget::new(instance);
        let ids = target.resolve(&self.connected(), true)?;
        let replies = self.forward(ids, |_| Ok(request.clone())).await;
        Self::gather(&target, replies)
    }

    // reads the file of `path` or takes `text`, checks it with the features of each target, then starts it there
    pub async fn run_control_file(
        &self,
        instance: Option<&str>,
        path: Option<String>,
        text: Option<String>,
    ) -> Result<Value, PortError> {
        let text = self.text(path, text).await?;
        let target = InstanceTarget::new(instance);
        let ids = target.resolve(&self.connected(), true)?;
        let registry = Arc::clone(self.forwarder.registry());
        let defaults = RunDefaults::default();
        // the check resolves join hosts, so it runs off the async threads, one task for each target
        let mut checks = Vec::new();
        for name in ids {
            let id = InstanceId::new(name.clone());
            let check = registry.get(&id).map(|member| member.features);
            let text = text.clone();
            let resolver = Arc::clone(&self.resolver);
            let task = tokio::task::spawn_blocking(move || match check {
                None => Err(PortError::UnknownInstance(id.to_string())),
                Some(features) => resolver.load_control(&text, &defaults, &features).map(|_| ()),
            });
            checks.push((name, task));
        }
        let mut refused = std::collections::HashMap::new();
        for (name, task) in checks {
            let verdict = task.await.unwrap_or_else(|_| Err(PortError::Io("the control file check failed".to_owned())));
            if let Err(error) = verdict {
                refused.insert(name, error);
            }
        }
        let ids = target.resolve(&self.connected(), true)?;
        let replies = self
            .forward(ids, |id| match refused.get(id.as_str()) {
                Some(error) => Err(error.clone()),
                None => Ok(MemberRequest::RunControlFile(text.clone())),
            })
            .await;
        Self::gather(&target, replies)
    }

    // a live action on one instance; a join host is resolved here, off the async threads, before it is forwarded
    pub async fn live(&self, instance: Option<&str>, action: LiveActionDto) -> Result<Value, PortError> {
        if matches!(action, LiveActionDto::JoinServer { .. }) {
            let probe = action.clone();
            let resolver = Arc::clone(&self.resolver);
            tokio::task::spawn_blocking(move || resolver.live(&probe).map(|_| ()))
                .await
                .unwrap_or_else(|_| Err(PortError::Io("the host lookup failed".to_owned())))?;
        }
        self.one(instance, MemberRequest::Live(action)).await
    }

    async fn text(&self, path: Option<String>, text: Option<String>) -> Result<String, PortError> {
        match (path, text) {
            (Some(_), Some(_)) => Err(PortError::Bad("give path or text, not both".to_owned())),
            (None, Some(text)) => Ok(text),
            (Some(path), None) => {
                let folder = self.folder.clone();
                tokio::task::spawn_blocking(move || {
                    let file = folder.resolve(&path);
                    std::fs::read_to_string(&file)
                        .map_err(|error| PortError::Io(format!("{} could not be read: {error}", file.display())))
                })
                .await
                .unwrap_or_else(|_| Err(PortError::Io("the control file read failed".to_owned())))
            }
            (None, None) => Err(PortError::Bad("give the path of a control file, or its text".to_owned())),
        }
    }

    // forwards the request `make` gives for each id at once; an id whose request `make` refuses is not sent
    async fn forward(
        &self,
        ids: Vec<String>,
        make: impl Fn(&InstanceId) -> Result<MemberRequest, PortError>,
    ) -> Vec<(String, Result<Value, PortError>)> {
        let mut sends = JoinSet::new();
        let mut refused = Vec::new();
        for (order, name) in ids.into_iter().enumerate() {
            let id = InstanceId::new(name.clone());
            match make(&id) {
                Ok(request) => {
                    let forwarder = Arc::clone(&self.forwarder);
                    sends.spawn(async move {
                        let reply = forwarder.send(&id, request).await.and_then(Self::value);
                        (order, name, reply)
                    });
                }
                Err(error) => refused.push((order, name, Err(error))),
            }
        }
        let mut all = sends.join_all().await;
        all.extend(refused);
        all.sort_by_key(|(order, _, _)| *order);
        all.into_iter().map(|(_, name, reply)| (name, reply)).collect()
    }

    fn gather(
        target: &InstanceTarget,
        mut replies: Vec<(String, Result<Value, PortError>)>,
    ) -> Result<Value, PortError> {
        if *target != InstanceTarget::All {
            return replies.pop().map_or_else(|| Err(PortError::UnknownInstance(String::new())), |(_, reply)| reply);
        }
        let results: Vec<InstanceResult> = replies
            .into_iter()
            .map(|(name, reply)| InstanceResult::new(name, reply.map_err(|error| error.to_string())))
            .collect();
        serde_json::to_value(results).map_err(|error| PortError::Io(error.to_string()))
    }

    fn value(reply: MemberReply) -> Result<Value, PortError> {
        match reply {
            MemberReply::Run(run) => {
                serde_json::to_value(RunStarted { run }).map_err(|error| PortError::Io(error.to_string()))
            }
            MemberReply::Done => Ok(json!({ "ok": true })),
            MemberReply::Status(status) => {
                serde_json::to_value(*status).map_err(|error| PortError::Io(error.to_string()))
            }
            MemberReply::Failed(error) => Err(error),
        }
    }

    fn connected(&self) -> Vec<String> {
        self.forwarder.registry().ids()
    }
}
