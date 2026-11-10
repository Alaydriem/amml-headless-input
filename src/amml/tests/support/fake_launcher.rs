use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use alaydriem_headless_input::traits::HubLauncher;
use headless_input_mcp::HubRole;

// a launcher whose nodes never end on their own: a stop always takes its whole wait, as a hub that ignores the
// cancel does. A start takes `start_delay`. `starts` counts the starts begun, `live` the nodes started and not yet
// stopped
pub struct FakeLauncher {
    pub starts: Arc<AtomicUsize>,
    pub live: Arc<AtomicUsize>,
    start_delay: Duration,
}

impl FakeLauncher {
    pub fn new(start_delay: Duration) -> Self {
        Self { starts: Arc::new(AtomicUsize::new(0)), live: Arc::new(AtomicUsize::new(0)), start_delay }
    }
}

impl HubLauncher for FakeLauncher {
    type Node = u16;

    fn start(&self, port: u16) -> u16 {
        self.starts.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.start_delay);
        self.live.fetch_add(1, Ordering::SeqCst);
        port
    }

    fn stop(&self, _node: u16, wait: Duration) {
        std::thread::sleep(wait);
        self.live.fetch_sub(1, Ordering::SeqCst);
    }

    fn role(&self, _node: &u16) -> HubRole {
        HubRole::Hub
    }
}
