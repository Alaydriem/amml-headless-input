use std::io;
use std::time::Duration;

use tokio::runtime::{Builder, Handle, Runtime};

// The Tokio runtime of the MCP hub: WORKERS worker threads named THREAD_NAME, and at most MAX_BLOCKING threads for the
// port calls, which block. It starts in enable and stops in disable, never in load
pub struct HeadlessTokio {
    runtime: Runtime,
}

impl HeadlessTokio {
    pub const WORKERS: usize = 2;
    pub const MAX_BLOCKING: usize = 16;
    pub const THREAD_NAME: &'static str = "headless-input";
    // the longest `stop` waits for the tasks; the ones still running are dropped
    pub const STOP_WAIT: Duration = Duration::from_millis(500);

    pub fn start() -> io::Result<Self> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(Self::WORKERS)
            .max_blocking_threads(Self::MAX_BLOCKING)
            .thread_name(Self::THREAD_NAME)
            .enable_all()
            .build()?;
        Ok(Self { runtime })
    }

    pub fn handle(&self) -> Handle {
        self.runtime.handle().clone()
    }

    // call it outside the runtime
    pub fn stop(self) {
        self.runtime.shutdown_timeout(Self::STOP_WAIT);
    }
}
