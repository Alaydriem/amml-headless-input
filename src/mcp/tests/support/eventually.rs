use std::time::{Duration, Instant};

// Waits for a condition with a time-out, looking every 20 ms.
pub struct Eventually;

impl Eventually {
    pub const STEP: Duration = Duration::from_millis(20);

    // true once `check` is true; false when `limit` passes first
    pub async fn wait(limit: Duration, mut check: impl FnMut() -> bool) -> bool {
        let start = Instant::now();
        loop {
            if check() {
                return true;
            }
            if start.elapsed() >= limit {
                return false;
            }
            tokio::time::sleep(Self::STEP).await;
        }
    }
}
