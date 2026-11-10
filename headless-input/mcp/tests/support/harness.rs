use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use headless_input_mcp::token::TokenStore;
use headless_input_mcp::{HubNode, HubRole, InstanceId};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{RoleClient, ServiceExt};
use serde_json::Value;
use tempfile::TempDir;

use super::{Eventually, FakePort, FreePort};

// A hub node on a free loopback port, over a `FakePort`, with a token store in a temp folder that holds one token
// (`token`). More nodes can join it with `join`.
pub struct Harness {
    pub fake: Arc<FakePort>,
    pub node: Option<HubNode>,
    pub token: String,
    pub dir: TempDir,
    port: u16,
}

impl Harness {
    pub const HUB_ID: &'static str = "alpha";
    pub const WAIT: Duration = Duration::from_secs(10);

    // must be called inside a multi-thread tokio runtime
    pub async fn start(fake: FakePort) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let token = TokenStore::load(dir.path()).issue("test").expect("token issued");
        let port = FreePort::pick();
        let fake = Arc::new(fake);
        let node = Self::node(port, Self::HUB_ID, Arc::clone(&fake), dir.path());
        assert!(Eventually::wait(Self::WAIT, || node.role() == HubRole::Hub).await, "the node did not become hub");
        Self { fake, node: Some(node), token, dir, port }
    }

    // a node with id `id` on `port`, sharing the token folder `dir`
    pub fn node(port: u16, id: &str, fake: Arc<FakePort>, dir: &Path) -> HubNode {
        HubNode::start(&tokio::runtime::Handle::current(), port, fake, InstanceId::new(id), TokenStore::load(dir))
    }

    // another node on the hub port; returns once the hub lists it
    pub async fn join(&self, id: &str, fake: Arc<FakePort>) -> HubNode {
        let node = Self::node(self.port, id, fake, self.dir.path());
        let hub = self.node.as_ref().expect("hub node");
        assert!(
            Eventually::wait(Self::WAIT, || hub.members().contains(&id.to_owned())).await,
            "{id} did not register: {:?}",
            hub.members()
        );
        node
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // a fresh store over the shared token file
    pub fn tokens(&self) -> TokenStore {
        TokenStore::load(self.dir.path())
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }

    pub fn pair_url(&self) -> String {
        format!("http://127.0.0.1:{}/pair", self.port)
    }

    pub async fn client(&self) -> RunningService<RoleClient, ()> {
        let transport = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(self.url()).auth_header(self.token.clone()),
        );
        ().serve(transport).await.expect("client connects")
    }

    pub async fn call(&self, name: &'static str, args: Value) -> CallToolResult {
        let client = self.client().await;
        let mut params = CallToolRequestParams::new(name);
        if let Value::Object(map) = args {
            params = params.with_arguments(map);
        }
        let result = client.call_tool(params).await.expect("tool call");
        let _ = client.cancel().await;
        result
    }

    pub fn text(result: &CallToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|block| block.as_text())
            .map(|text| text.text.clone())
            .collect::<Vec<_>>()
            .join("")
    }

    // stops a node off the runtime workers
    pub async fn stop_node(node: HubNode) {
        tokio::task::spawn_blocking(move || node.stop()).await.expect("stop");
    }

    pub async fn stop(mut self) {
        if let Some(node) = self.node.take() {
            Self::stop_node(node).await;
        }
    }
}
