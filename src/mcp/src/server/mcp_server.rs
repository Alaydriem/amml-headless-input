use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::{BearerGuard, HeadlessTools, PairRoute};
use crate::hub::{ControlFolder, Forwarder, MemberRoute};
use crate::port::{HeadlessPort, HostResolver};
use crate::token::TokenStore;

// The hub's HTTP server on the hub port: MCP Streamable HTTP at `/mcp` and the member routes, both behind the bearer
// check, and `POST /pair` and `GET /member/hello` with no bearer check.
pub struct McpServer {
    address: SocketAddr,
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

impl McpServer {
    pub const PATH: &'static str = "/mcp";
    // the longest `stop` waits for the serve task to end before it aborts the task
    pub const STOP_WAIT: Duration = Duration::from_secs(2);

    // serves on `listener`; call inside the Tokio runtime that serves. `/mcp` and `/member/*` take the tokens of
    // `tokens`; `/pair` adds to it
    pub fn start<P: HeadlessPort>(
        listener: std::net::TcpListener,
        local: Arc<P>,
        forwarder: Arc<Forwarder>,
        tokens: Arc<TokenStore>,
        resolver: Arc<HostResolver>,
    ) -> anyhow::Result<McpServer> {
        let address = listener.local_addr()?;
        let cancel = CancellationToken::new();
        let folder = ControlFolder::new(Arc::clone(&local));
        let pair = PairRoute::new_shared(local, Arc::clone(&tokens));
        let members = MemberRoute::new_shared(Arc::clone(&forwarder), cancel.clone());
        let config = StreamableHttpServerConfig::default().with_cancellation_token(cancel.child_token());
        let service: StreamableHttpService<HeadlessTools, LocalSessionManager> = StreamableHttpService::new(
            move || Ok(HeadlessTools::new(Arc::clone(&forwarder), folder.clone(), Arc::clone(&resolver))),
            Arc::new(LocalSessionManager::default()),
            config,
        );
        let guard = Arc::new(BearerGuard::new(tokens));
        let guarded = axum::Router::new()
            .nest_service(Self::PATH, service)
            .merge(MemberRoute::guarded(members))
            .layer(axum::middleware::from_fn_with_state(guard, BearerGuard::check));
        let open = axum::Router::new()
            .route(PairRoute::<P>::PATH, axum::routing::post(PairRoute::<P>::handle))
            .with_state(pair)
            .route(MemberRoute::HELLO, axum::routing::get(MemberRoute::hello));
        let router = guarded.merge(open);

        let listener = tokio::net::TcpListener::from_std(listener)?;
        let shutdown = cancel.clone();
        let task = tokio::spawn(async move {
            let served = axum::serve(listener, router).with_graceful_shutdown(shutdown.cancelled_owned()).await;
            if let Err(error) = served {
                curia::error!("the MCP server stopped", { error: error.to_string() });
            }
        });
        Ok(McpServer { address, cancel, task })
    }

    // the bound address (always `127.0.0.1`)
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    // ends all sessions and stops the server. Waits up to `STOP_WAIT` for the serve task to end, so the port is free
    // when this returns; after that the task is aborted
    pub async fn stop(mut self) {
        self.cancel.cancel();
        if tokio::time::timeout(Self::STOP_WAIT, &mut self.task).await.is_err() {
            self.task.abort();
            let _ = (&mut self.task).await;
        }
    }
}

impl Drop for McpServer {
    // a dropped server stops serving; the port is free soon after (use `stop` to wait for it)
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
