use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use headless_input_core::structs::Capabilities;
use tokio_util::sync::CancellationToken;

use super::{
    ExclusiveListener, Forwarder, HubRole, InstanceId, Jitter, LinkEnd, LocalMember, MemberInfo, MemberLink,
    MemberRegistry, RoleCell,
};
use crate::port::{HeadlessPort, HostResolver};
use crate::server::McpServer;
use crate::token::TokenStore;

// The task of one hub node: holds the hub port and serves, or links to the hub as a member, and takes the port over
// when the hub goes. Ends when the node stops. When the hub answers that another game has its instance id, the node
// takes the id with its process id after it for the rest of its life, with one warning.
pub struct HubSupervisor<P: HeadlessPort> {
    port: u16,
    local: Arc<P>,
    id: InstanceId,
    tokens: Arc<TokenStore>,
    role: Arc<RoleCell>,
    serving: Arc<Mutex<Option<Arc<Forwarder>>>>,
    cancel: CancellationToken,
    client: reqwest::Client,
    resolver: Arc<HostResolver>,
}

impl<P: HeadlessPort> HubSupervisor<P> {
    pub const PRUNE_EVERY: Duration = Duration::from_secs(1);
    // broken answers in a row before the program on the port is taken as not a hub; a hub that shuts down gives one
    // or two
    pub const GARBLED_LIMIT: u32 = 10;

    pub fn new(
        port: u16,
        local: Arc<P>,
        id: InstanceId,
        tokens: Arc<TokenStore>,
        role: Arc<RoleCell>,
        serving: Arc<Mutex<Option<Arc<Forwarder>>>>,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            port,
            local,
            id,
            tokens,
            role,
            serving,
            cancel,
            client: MemberLink::<P>::client(),
            resolver: HostResolver::new_shared(),
        }
    }

    // `first` is the listener when the bind in `HubNode::start` won the port
    pub async fn run(mut self, first: Option<TcpListener>) {
        let mut listener = first;
        let mut garbled = 0u32;
        let mut renamed = false;
        while !self.cancel.is_cancelled() {
            let bound = listener.take().or_else(|| ExclusiveListener::bind(self.port).ok());
            if let Some(bound) = bound {
                self.serve(bound).await;
                break;
            }
            let link = MemberLink::new(
                self.client.clone(),
                self.port,
                Arc::clone(&self.local),
                Arc::clone(&self.resolver),
                self.id.clone(),
                Arc::clone(&self.tokens),
                Arc::clone(&self.role),
                self.cancel.clone(),
            );
            let end = link.run().await;
            garbled = if matches!(end, LinkEnd::Garbled) { garbled + 1 } else { 0 };
            match end {
                LinkEnd::Lost => self.pause(Jitter::wait()).await,
                LinkEnd::Garbled if garbled < Self::GARBLED_LIMIT => self.pause(Jitter::wait()).await,
                LinkEnd::Taken if !renamed => {
                    let distinct = self.id.with_pid(std::process::id());
                    curia::warn!("another game uses this instance id; this game registers under a distinct id", {
                        id: self.id.to_string(),
                        distinct: distinct.to_string()
                    });
                    self.id = distinct;
                    renamed = true;
                }
                LinkEnd::Taken | LinkEnd::Refused => self.pause(MemberLink::<P>::RETRY).await,
                LinkEnd::Foreign | LinkEnd::Garbled => {
                    self.role.set(HubRole::Off);
                    curia::error!(
                        "the program on the MCP hub port is not a headless-input hub; MCP stays off. Free the port or change mcp.port",
                        { port: self.port, broken_answers: garbled }
                    );
                    self.cancel.cancelled().await;
                    break;
                }
                LinkEnd::Stopped => break,
                LinkEnd::TakeOver(taken) => listener = Some(taken),
            }
        }
        self.role.set(HubRole::Off);
    }

    async fn serve(&self, listener: TcpListener) {
        let registry = MemberRegistry::new_shared();
        let forwarder = Forwarder::new_shared(Arc::clone(&registry), Forwarder::TIMEOUT);
        let local = Arc::clone(&self.local);
        let features = tokio::task::spawn_blocking(move || local.features())
            .await
            .unwrap_or(Capabilities::new(false, false, false, false, false, false));
        let own = MemberInfo::new(self.id.clone(), std::process::id(), features, true);
        if let Err(error) = registry.register(own) {
            curia::error!("the hub could not register its own instance", { error: error.to_string() });
        }
        let server = match McpServer::start(
            listener,
            Arc::clone(&self.local),
            Arc::clone(&forwarder),
            Arc::clone(&self.tokens),
            Arc::clone(&self.resolver),
        ) {
            Ok(server) => server,
            Err(error) => {
                curia::error!("the MCP hub could not serve", { port: self.port, error: error.to_string() });
                self.role.set(HubRole::Off);
                self.cancel.cancelled().await;
                return;
            }
        };
        tokio::spawn(LocalMember::run(
            Arc::clone(&self.local),
            self.id.clone(),
            Arc::clone(&forwarder),
            Arc::clone(&self.resolver),
            self.cancel.child_token(),
        ));
        tokio::spawn(Self::prune(Arc::clone(&registry), self.cancel.child_token()));
        self.set_serving(Some(Arc::clone(&forwarder)));
        self.role.set(HubRole::Hub);
        curia::info!("this instance is the MCP hub", { port: self.port, instance: self.id.to_string() });
        self.cancel.cancelled().await;
        self.set_serving(None);
        server.stop().await;
    }

    async fn prune(registry: Arc<MemberRegistry>, cancel: CancellationToken) {
        let mut ticker = tokio::time::interval(Self::PRUNE_EVERY);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = ticker.tick() => {
                    for id in registry.prune(MemberRegistry::STALE) {
                        curia::info!("a member stopped answering and was dropped", { instance: id.to_string() });
                    }
                }
            }
        }
    }

    async fn pause(&self, wait: Duration) {
        tokio::select! {
            _ = self.cancel.cancelled() => {}
            _ = tokio::time::sleep(wait) => {}
        }
    }

    fn set_serving(&self, forwarder: Option<Arc<Forwarder>>) {
        *self.serving.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = forwarder;
    }
}
