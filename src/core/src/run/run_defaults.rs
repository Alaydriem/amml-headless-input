use crate::config::HeadlessConfig;
use crate::control::Ticks;

// The settings a control file and the run supervisor fall back on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunDefaults {
    // the timeout of a goal step that names none
    pub goal_timeout: Ticks,
    // wall-clock time with no game tick before an active run is aborted
    pub watchdog: Ticks,
    // wall-clock time with no live call before a live session ends
    pub live_idle: Ticks,
}

impl RunDefaults {
    pub fn new(config: &HeadlessConfig) -> Self {
        Self {
            goal_timeout: Ticks(config.goal_timeout_ticks),
            watchdog: Ticks(config.watchdog_ticks),
            live_idle: Ticks(config.live_idle_ticks),
        }
    }
}

impl Default for RunDefaults {
    fn default() -> Self {
        Self { goal_timeout: Ticks(600), watchdog: Ticks(100), live_idle: Ticks(600) }
    }
}
