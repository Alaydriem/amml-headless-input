use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use headless_input_core::run::{Command, CommandQueue};
use headless_input_mcp::port::PortError;

use crate::services::CommandReply;

// The commands the local port queued for the game thread, each with the reply its caller waits on, at most
// `CommandQueue::CAPACITY`. A push to a full inbox is refused as Busy and the queued commands stay. The game thread
// drains it once a frame into buffers it reuses. A closed inbox (the mod is disabled) answers every queued command
// with a refusal and refuses new ones, until it is opened again
pub struct CommandInbox {
    queued: Mutex<(CommandQueue, VecDeque<Arc<CommandReply>>, bool)>,
}

impl CommandInbox {
    pub const CLOSED: &'static str = "the mod was disabled";

    pub fn new() -> Self {
        Self { queued: Mutex::new((CommandQueue::new(), VecDeque::with_capacity(CommandQueue::CAPACITY), false)) }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn push(&self, command: Command, reply: Arc<CommandReply>) -> Result<(), PortError> {
        let mut queued = self.lock();
        if queued.2 {
            return Err(PortError::Refused(Self::CLOSED.to_owned()));
        }
        queued.0.try_push(command).map_err(|_| PortError::Busy)?;
        queued.1.push_back(reply);
        Ok(())
    }

    // moves every queued command and its reply into the buffers, oldest first
    pub fn drain(&self, commands: &mut Vec<Command>, replies: &mut Vec<Arc<CommandReply>>) {
        let mut queued = self.lock();
        if queued.0.is_empty() {
            return;
        }
        queued.0.drain(commands);
        replies.extend(queued.1.drain(..));
    }

    // refuses new commands and answers every queued one with a refusal, so no caller waits on a game that is gone
    pub fn close(&self) {
        let mut commands = Vec::new();
        let mut replies = Vec::new();
        {
            let mut queued = self.lock();
            queued.2 = true;
            queued.0.drain(&mut commands);
            replies.extend(queued.1.drain(..));
        }
        for reply in replies {
            reply.answer(|| Err(PortError::Refused(Self::CLOSED.to_owned())));
        }
    }

    pub fn open(&self) {
        self.lock().2 = false;
    }

    pub fn len(&self) -> usize {
        self.lock().0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> MutexGuard<'_, (CommandQueue, VecDeque<Arc<CommandReply>>, bool)> {
        self.queued.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for CommandInbox {
    fn default() -> Self {
        Self::new()
    }
}
