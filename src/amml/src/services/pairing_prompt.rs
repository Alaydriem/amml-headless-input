use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::types::PromptState;

// Asks the person in the hub's game whether an MCP client may pair. The question is answered by the setting
// `mcp.allow_pairing`: it allows the question only when it turns on while the question waits, within ANSWER_WAIT; no
// switch denies. A switch with no question waiting allows nothing. The asker blocks on a port thread; the game thread
// passes the switch
pub struct PairingPrompt {
    state: Mutex<PromptState>,
    answered: Condvar,
}

impl PairingPrompt {
    pub const ANSWER_WAIT: Duration = Duration::from_secs(60);

    pub fn new() -> Self {
        Self { state: Mutex::new(PromptState::new()), answered: Condvar::new() }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    // blocks up to ANSWER_WAIT; true only when the person allowed it
    pub fn ask(&self, client: &str) -> bool {
        self.ask_within(client, Self::ANSWER_WAIT)
    }

    // as `ask`, waiting up to `wait`. A second question while one waits is denied
    pub fn ask_within(&self, client: &str, wait: Duration) -> bool {
        curia::info!("an MCP client asks to pair; turn on Allow MCP pairing in the mod settings to allow it", {
            client: client.to_owned(),
            seconds: wait.as_secs()
        });
        let mut state = self.lock();
        if state.asking || state.closed {
            return false;
        }
        state.asking = true;
        state.answer = None;
        let deadline = Instant::now() + wait.min(Self::ANSWER_WAIT);
        let allowed = loop {
            if let Some(answer) = state.answer {
                break answer;
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()).filter(|left| !left.is_zero()) else {
                break false;
            };
            state = self.answered.wait_timeout(state, left).unwrap_or_else(PoisonError::into_inner).0;
        };
        state.asking = false;
        state.answer = None;
        drop(state);
        if allowed {
            curia::info!("the MCP pairing was allowed", { client: client.to_owned() });
        } else {
            curia::warn!("the MCP pairing was denied", { client: client.to_owned() });
        }
        allowed
    }

    // the setting turned on, on the game thread: it allows the question that waits, and does nothing when none does
    pub fn allow(&self) {
        let mut state = self.lock();
        if state.asking && state.answer.is_none() {
            state.answer = Some(true);
            self.answered.notify_all();
        }
    }

    // disable: the waiting question is denied now, and every later one at once, until `reopen`
    pub fn cancel(&self) {
        let mut state = self.lock();
        state.closed = true;
        if state.asking && state.answer.is_none() {
            state.answer = Some(false);
            self.answered.notify_all();
        }
    }

    // enable: questions wait for an answer again
    pub fn reopen(&self) {
        self.lock().closed = false;
    }

    pub fn is_asking(&self) -> bool {
        self.lock().asking
    }

    fn lock(&self) -> MutexGuard<'_, PromptState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for PairingPrompt {
    fn default() -> Self {
        Self::new()
    }
}
