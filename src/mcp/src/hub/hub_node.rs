use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::{ExclusiveListener, Forwarder, HubRole, HubSupervisor, InstanceId, RoleCell};
use crate::port::HeadlessPort;
use crate::token::TokenStore;

// One instance's place in the hub: the instance that binds the hub port serves MCP for every instance, the others
// register with it as members, and one of them takes over when it exits.
pub struct HubNode {
    port: u16,
    role: Arc<RoleCell>,
    serving: Arc<Mutex<Option<Arc<Forwarder>>>>,
    cancel: CancellationToken,
    task: JoinHandle<()>,
    // the task drops its sender when it ends, so `recv_timeout` returns at that moment
    ended: Receiver<()>,
}

impl HubNode {
    // the longest `stop` waits for the node task to end before it aborts the task
    pub const STOP_WAIT: Duration = Duration::from_secs(4);

    // tries the bind on `127.0.0.1:<port>` at once; on success this node is hub, else it links to the hub
    pub fn start<P: HeadlessPort>(
        runtime: &Handle,
        port: u16,
        local: Arc<P>,
        id: InstanceId,
        tokens: TokenStore,
    ) -> HubNode {
        let first = ExclusiveListener::bind(port).ok();
        let role = Arc::new(RoleCell::new(if first.is_some() { HubRole::Hub } else { HubRole::Off }));
        let serving = Arc::new(Mutex::new(None));
        let cancel = CancellationToken::new();
        let supervisor = HubSupervisor::new(
            port,
            local,
            id,
            Arc::new(tokens),
            Arc::clone(&role),
            Arc::clone(&serving),
            cancel.clone(),
        );
        let (alive, ended) = std::sync::mpsc::channel::<()>();
        let task = runtime.spawn(async move {
            let _alive: Sender<()> = alive;
            supervisor.run(first).await;
        });
        HubNode { port, role, serving, cancel, task, ended }
    }

    pub fn role(&self) -> HubRole {
        self.role.get()
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // the ids of the instances this node serves, itself among them; empty unless this node is hub
    pub fn members(&self) -> Vec<String> {
        self.serving
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|forwarder| forwarder.registry().ids())
            .unwrap_or_default()
    }

    // the ids of the members whose poll is open on this hub now; empty unless this node is hub
    pub fn polling(&self) -> Vec<String> {
        self.serving
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|forwarder| forwarder.registry().polling())
            .unwrap_or_default()
    }

    // a member unregisters, a hub stops serving and frees the port. Waits up to `STOP_WAIT`, then aborts the task.
    // Blocks: call it from a thread that is not the only worker of the runtime
    pub fn stop(self) {
        self.stop_within(Self::STOP_WAIT);
    }

    // as `stop`, waiting up to `wait` for the node task before it is aborted
    pub fn stop_within(self, wait: Duration) {
        self.cancel.cancel();
        if self.ended.recv_timeout(wait).is_err() {
            self.task.abort();
        }
    }
}

impl Drop for HubNode {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
