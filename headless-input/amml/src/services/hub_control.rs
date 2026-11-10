use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use headless_input_mcp::HubRole;
use tokio::runtime::Handle;

use crate::services::NodeLauncher;
use crate::traits::HubLauncher;

// Keeps this instance's hub node in step with mcp.enabled and mcp.port: starts it, moves it to a new port or stops
// it, and starts it again when it has had no MCP role for RETRY (the serve failed, or the hub refused it). Starting
// and stopping block, so they run on a blocking thread of the runtime, one switch at a time; `follow` never blocks.
// `stop` returns within SWITCH_WAIT + NODE_STOP_WAIT, and a node a switch under way starts after it is stopped by
// that switch
pub struct HubControl<L: HubLauncher = NodeLauncher> {
    handle: Handle,
    launcher: Arc<L>,
    node: Arc<Mutex<Option<L::Node>>>,
    switching: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
    // the port the node was last started on; None while MCP is off
    running: Option<u16>,
    started: Instant,
}

impl<L: HubLauncher> HubControl<L> {
    pub const RETRY: Duration = Duration::from_secs(2);
    // the longest `stop` waits for a switch under way
    pub const SWITCH_WAIT: Duration = Duration::from_millis(500);
    // the longest a node is given to end when it is stopped
    pub const NODE_STOP_WAIT: Duration = Duration::from_millis(1500);

    pub fn new(handle: Handle, launcher: L) -> Self {
        Self {
            handle,
            launcher: Arc::new(launcher),
            node: Arc::new(Mutex::new(None)),
            switching: Arc::new(AtomicBool::new(false)),
            stopping: Arc::new(AtomicBool::new(false)),
            running: None,
            started: Instant::now(),
        }
    }

    // once a frame on the game thread, with the settings now in force
    pub fn follow(&mut self, enabled: bool, port: u16, now: Instant) {
        if self.switching.load(Ordering::SeqCst) || self.stopping.load(Ordering::SeqCst) {
            return;
        }
        let wanted = enabled.then_some(port);
        if wanted != self.running {
            self.switch(wanted, now);
        } else if wanted.is_some() && now.saturating_duration_since(self.started) >= Self::RETRY && self.without_role()
        {
            curia::warn!("the MCP hub node has no role; it is started again", { port: port });
            self.switch(wanted, now);
        }
    }

    // the role of the node; None without a node, or while a switch holds it
    pub fn role(&self) -> Option<HubRole> {
        let node = self.node.try_lock().ok()?;
        node.as_ref().map(|node| self.launcher.role(node))
    }

    pub fn is_switching(&self) -> bool {
        self.switching.load(Ordering::SeqCst)
    }

    // disable: no switch starts after this call, a switch under way is waited for up to SWITCH_WAIT, then the stored
    // node is stopped within NODE_STOP_WAIT, which frees the hub port
    pub fn stop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + Self::SWITCH_WAIT;
        while self.switching.load(Ordering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let node = self.node.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(node) = node {
            self.launcher.stop(node, Self::NODE_STOP_WAIT);
        }
        self.running = None;
    }

    // a node that is stored and has no MCP role, or no node at all
    fn without_role(&self) -> bool {
        match self.node.try_lock() {
            Ok(node) => node.as_ref().is_none_or(|node| self.launcher.role(node) == HubRole::Off),
            Err(_) => false,
        }
    }

    fn switch(&mut self, wanted: Option<u16>, now: Instant) {
        self.running = wanted;
        self.started = now;
        self.switching.store(true, Ordering::SeqCst);
        let node = Arc::clone(&self.node);
        let switching = Arc::clone(&self.switching);
        let stopping = Arc::clone(&self.stopping);
        let launcher = Arc::clone(&self.launcher);
        drop(self.handle.spawn_blocking(move || {
            let old = node.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(old) = old {
                launcher.stop(old, Self::NODE_STOP_WAIT);
            }
            if let Some(port) = wanted
                && !stopping.load(Ordering::SeqCst)
            {
                let started = launcher.start(port);
                let mut slot = node.lock().unwrap_or_else(PoisonError::into_inner);
                if stopping.load(Ordering::SeqCst) {
                    drop(slot);
                    launcher.stop(started, Self::NODE_STOP_WAIT);
                } else {
                    *slot = Some(started);
                }
            }
            switching.store(false, Ordering::SeqCst);
        }));
    }
}
