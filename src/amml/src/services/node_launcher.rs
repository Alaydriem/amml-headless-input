use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use headless_input_mcp::{HubNode, HubRole, InstanceId, TokenStore};
use tokio::runtime::Handle;

use crate::services::LocalPort;
use crate::traits::HubLauncher;

// Makes the real hub node of this instance on the Tokio runtime, over the local port, with the shared token file
pub struct NodeLauncher {
    handle: Handle,
    local: Arc<LocalPort>,
    id: InstanceId,
    tokens: PathBuf,
}

impl NodeLauncher {
    // `tokens` is the folder of the shared token file
    pub fn new(handle: Handle, local: Arc<LocalPort>, id: InstanceId, tokens: &Path) -> Self {
        Self { handle, local, id, tokens: tokens.to_path_buf() }
    }
}

impl HubLauncher for NodeLauncher {
    type Node = HubNode;

    fn start(&self, port: u16) -> HubNode {
        HubNode::start(&self.handle, port, Arc::clone(&self.local), self.id.clone(), TokenStore::load(&self.tokens))
    }

    fn stop(&self, node: HubNode, wait: Duration) {
        node.stop_within(wait);
    }

    fn role(&self, node: &HubNode) -> HubRole {
        node.role()
    }
}
