use std::sync::Arc;

use crate::command::CommandClient;
use crate::errors::CommandError;

// Sends the commands the input frames asked for, once for each request. A tick asks with `request`; the next game
// frame calls `pump`, which sends every asked command in order. The queue holds at most `CAPACITY` requests: a request
// made while it is full is dropped, so the earlier commands keep their order. A command that cannot be sent is
// dropped with the rest of the queue, and `pump` answers why. Game thread.
pub struct CommandService {
    pending: Vec<Arc<str>>,
}

impl CommandService {
    // the most commands that wait for a frame; two ticks come before a frame only on a very slow game
    pub const CAPACITY: usize = 8;

    pub fn new() -> Self {
        Self { pending: Vec::with_capacity(Self::CAPACITY) }
    }

    // asks for `text` to be sent on the next pump; false when the queue is full and the request was dropped
    pub fn request(&mut self, text: &Arc<str>) -> bool {
        if self.pending.len() >= Self::CAPACITY {
            return false;
        }
        self.pending.push(Arc::clone(text));
        true
    }

    // drops every waiting request, for a run that ended
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    // once per frame. It calls the game only while a request waits. The first command the client cannot send ends
    // the pump with the reason and drops what is left
    pub fn pump(&mut self, client: &impl CommandClient) -> Result<(), CommandError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if !client.has_local_player() {
            self.pending.clear();
            return Err(CommandError::NoPlayer);
        }
        for text in self.pending.drain(..) {
            if !client.send_command(&text) {
                return Err(CommandError::NotSent);
            }
            curia::info!("a command was sent to the server", { text: text.to_string() });
        }
        Ok(())
    }
}

impl Default for CommandService {
    fn default() -> Self {
        Self::new()
    }
}
