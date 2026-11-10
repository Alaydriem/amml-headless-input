use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use super::{ExclusiveListener, HubRole, InstanceId, LinkEnd, MemberExecutor, MemberRoute, RoleCell};
use crate::dto::{FeaturesDto, Forwarded, HubHello, MemberRef, RegisterRequest, ReplyPost};
use crate::port::{HeadlessPort, HostResolver};
use crate::token::TokenStore;

// A member's connection to the hub: the handshake, the registration, the long poll for commands and the heartbeat.
// A hub that does not answer a heartbeat for `STALL_AFTER` is stalled; while stalled the link tries the bind each
// heartbeat and takes the port when the stalled process lets it go.
pub struct MemberLink<P: HeadlessPort> {
    client: reqwest::Client,
    base: String,
    hub_port: u16,
    port: Arc<P>,
    resolver: Arc<HostResolver>,
    id: InstanceId,
    tokens: Arc<TokenStore>,
    role: Arc<RoleCell>,
    cancel: CancellationToken,
}

impl<P: HeadlessPort> MemberLink<P> {
    pub const HEARTBEAT: Duration = Duration::from_secs(1);
    pub const STALL_AFTER: Duration = Duration::from_secs(5);
    pub const HELLO_WAIT: Duration = Duration::from_secs(2);
    pub const CALL_WAIT: Duration = Duration::from_secs(5);
    // longer than the poll wait of the hub
    pub const POLL_TIMEOUT: Duration = Duration::from_secs(30);
    // between a refused registration and the next try
    pub const RETRY: Duration = Duration::from_secs(2);

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: reqwest::Client,
        hub_port: u16,
        port: Arc<P>,
        resolver: Arc<HostResolver>,
        id: InstanceId,
        tokens: Arc<TokenStore>,
        role: Arc<RoleCell>,
        cancel: CancellationToken,
    ) -> Self {
        let base = format!("http://127.0.0.1:{hub_port}");
        Self { client, base, hub_port, port, resolver, id, tokens, role, cancel }
    }

    // the client the links use: no proxy, the hub is on loopback
    pub fn client() -> reqwest::Client {
        reqwest::Client::builder().no_proxy().build().unwrap_or_else(|_| reqwest::Client::new())
    }

    // one connection to the hub, from the handshake until it ends
    pub async fn run(&self) -> LinkEnd {
        if let Err(end) = self.hello().await {
            return end;
        }
        let token = match self.tokens.member_token() {
            Ok(token) => token,
            Err(error) => {
                curia::warn!("the member token could not be saved", { error: error.to_string() });
                return LinkEnd::Refused;
            }
        };
        if let Some(end) = self.register(&token).await {
            return end;
        }
        self.role.set(HubRole::Member);
        let end = tokio::select! {
            end = self.poll_loop(&token) => end,
            end = self.heartbeat_loop(&token) => end,
            _ = self.cancel.cancelled() => {
                self.unregister(&token).await;
                LinkEnd::Stopped
            }
        };
        self.role.set(HubRole::Off);
        end
    }

    // Ok for a headless-input hub of this protocol. A complete HTTP reply of any other kind is `Foreign`; no answer
    // (refused or timed out) is `Lost`; a broken answer (a reset, or bytes that are not HTTP) is `Garbled`, which a
    // hub that shuts down also gives
    async fn hello(&self) -> Result<(), LinkEnd> {
        let sent = self.client.get(self.url(MemberRoute::HELLO)).timeout(Self::HELLO_WAIT).send().await;
        let response = match sent {
            Ok(response) => response,
            Err(error) if error.is_connect() || error.is_timeout() => return Err(LinkEnd::Lost),
            Err(_) => return Err(LinkEnd::Garbled),
        };
        let status = response.status();
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => return Err(LinkEnd::Lost),
            Err(_) => return Err(LinkEnd::Garbled),
        };
        let ours =
            status == StatusCode::OK && serde_json::from_slice::<HubHello>(&body).is_ok_and(|hello| hello.is_ours());
        if ours { Ok(()) } else { Err(LinkEnd::Foreign) }
    }

    async fn register(&self, token: &str) -> Option<LinkEnd> {
        let request = RegisterRequest {
            id: self.id.to_string(),
            pid: std::process::id(),
            features: FeaturesDto::from(self.port.features()),
        };
        let sent = self.post(MemberRoute::REGISTER, token, &request, Self::CALL_WAIT).await;
        match sent {
            Ok(response) if response.status() == StatusCode::OK => None,
            Ok(response) if response.status() == StatusCode::CONFLICT => Some(LinkEnd::Taken),
            Ok(response) => {
                curia::warn!("the hub refused the registration", { status: response.status().as_u16() });
                Some(LinkEnd::Refused)
            }
            Err(_) => Some(LinkEnd::Lost),
        }
    }

    async fn poll_loop(&self, token: &str) -> LinkEnd {
        let url = self.url(MemberRoute::POLL);
        loop {
            let sent = self
                .client
                .get(&url)
                .bearer_auth(token)
                .query(&[("id", self.id.as_str())])
                .timeout(Self::POLL_TIMEOUT)
                .send()
                .await;
            let response = match sent {
                Ok(response) => response,
                Err(error) if error.is_timeout() => continue,
                Err(_) => return LinkEnd::Lost,
            };
            match response.status() {
                StatusCode::OK => {
                    let Ok(body) = response.bytes().await else {
                        return LinkEnd::Lost;
                    };
                    let Ok(forwarded) = serde_json::from_slice::<Forwarded>(&body) else {
                        continue;
                    };
                    let reply = MemberExecutor::execute_blocking(
                        Arc::clone(&self.port),
                        Arc::clone(&self.resolver),
                        forwarded.request,
                    )
                    .await;
                    let post = ReplyPost { id: self.id.to_string(), command: forwarded.command, reply };
                    if self.post(MemberRoute::REPLY, token, &post, Self::CALL_WAIT).await.is_err() {
                        return LinkEnd::Lost;
                    }
                }
                StatusCode::NO_CONTENT => {}
                _ => return LinkEnd::Lost,
            }
        }
    }

    async fn heartbeat_loop(&self, token: &str) -> LinkEnd {
        let member = MemberRef { id: self.id.to_string() };
        let mut answered = Instant::now();
        let mut ticker = tokio::time::interval(Self::HEARTBEAT);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            match self.post(MemberRoute::HEARTBEAT, token, &member, Self::HEARTBEAT).await {
                Ok(response) if response.status() == StatusCode::NO_CONTENT => {
                    answered = Instant::now();
                    self.role.set(HubRole::Member);
                }
                Ok(response) if matches!(response.status(), StatusCode::NOT_FOUND | StatusCode::UNAUTHORIZED) => {
                    return LinkEnd::Lost;
                }
                Ok(_) => {}
                Err(error) if error.is_connect() => return LinkEnd::Lost,
                Err(_) => {}
            }
            if answered.elapsed() >= Self::STALL_AFTER {
                if self.role.get() != HubRole::Stalled {
                    curia::warn!("the hub stopped answering; it is marked stalled", { port: self.hub_port });
                }
                self.role.set(HubRole::Stalled);
                if let Ok(listener) = ExclusiveListener::bind(self.hub_port) {
                    return LinkEnd::TakeOver(listener);
                }
            }
        }
    }

    async fn unregister(&self, token: &str) {
        let member = MemberRef { id: self.id.to_string() };
        let _ = self.post(MemberRoute::UNREGISTER, token, &member, Duration::from_millis(500)).await;
    }

    async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        token: &str,
        body: &T,
        wait: Duration,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let text = serde_json::to_string(body).unwrap_or_default();
        self.client
            .post(self.url(path))
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text)
            .timeout(wait)
            .send()
            .await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}
