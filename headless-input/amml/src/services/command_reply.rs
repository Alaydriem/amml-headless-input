use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use headless_input_core::run::RunId;
use headless_input_mcp::port::PortError;

use crate::types::ReplyState;

// The answer to one queued command: the port thread waits for it, the game thread gives it when it applies the
// command. A command whose asker stopped waiting is dropped unrun, so a timed-out call never starts a run later
pub struct CommandReply {
    state: Mutex<ReplyState>,
    ready: Condvar,
}

impl CommandReply {
    pub const NO_FRAME: &'static str = "the game ran no frame to take the command";
    // the longest any wait lasts, so the deadline cannot overflow
    const MAX_WAIT: Duration = Duration::from_secs(3600);

    pub fn new() -> Self {
        Self { state: Mutex::new(ReplyState::Waiting), ready: Condvar::new() }
    }

    // waits up to `wait` for the answer. With none by then the command is abandoned and the call times out
    pub fn wait(&self, wait: Duration) -> Result<RunId, PortError> {
        let deadline = Instant::now() + wait.min(Self::MAX_WAIT);
        let mut state = self.lock();
        loop {
            if let ReplyState::Answered(result) = &*state {
                return result.clone();
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()).filter(|left| !left.is_zero()) else {
                *state = ReplyState::Abandoned;
                return Err(PortError::Timeout(Self::NO_FRAME.to_owned()));
            };
            state = self.ready.wait_timeout(state, left).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    // runs `body` and gives its result, unless the asker stopped waiting; then `body` does not run. The asker cannot
    // give up while `body` runs
    pub fn answer(&self, body: impl FnOnce() -> Result<RunId, PortError>) {
        let mut state = self.lock();
        if *state == ReplyState::Abandoned {
            return;
        }
        *state = ReplyState::Answered(body());
        self.ready.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, ReplyState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for CommandReply {
    fn default() -> Self {
        Self::new()
    }
}
