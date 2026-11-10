use std::time::{Duration, Instant};

use crate::control::{Step, Ticks};
use crate::player::StepCursor;

// A join or leave under way. The game loads without ticks while it runs, so its timeout runs on the wall clock, and
// the screen rule, the world-left rule and the watchdog wait for it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Travel {
    since: Instant,
    limit: Duration,
    why: &'static str,
    // the file step it belongs to; None in a live session
    cursor: Option<StepCursor>,
}

impl Travel {
    // None for a step that is not a join or a leave
    pub fn new(step: &Step, since: Instant, cursor: Option<StepCursor>) -> Option<Self> {
        let (timeout, why): (Ticks, &'static str) = match step {
            Step::JoinServer { timeout, .. } => (*timeout, "join_server passed its timeout"),
            Step::LeaveWorld { timeout } => (*timeout, "leave_world passed its timeout"),
            _ => return None,
        };
        Some(Self { since, limit: timeout.duration(), why, cursor })
    }

    pub fn same_step(&self, cursor: &StepCursor) -> bool {
        self.cursor.is_some_and(|own| own.step == cursor.step && own.repeat_passes() == cursor.repeat_passes())
    }

    pub fn expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.since) >= self.limit
    }

    pub fn why(&self) -> &'static str {
        self.why
    }
}
