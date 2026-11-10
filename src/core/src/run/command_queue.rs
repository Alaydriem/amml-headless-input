use super::Command;

// The commands waiting for the game thread, at most CAPACITY. A push to a full queue is refused and the queued
// commands stay; the caller reports Busy.
#[derive(Debug)]
pub struct CommandQueue {
    items: Vec<Command>,
}

impl CommandQueue {
    pub const CAPACITY: usize = 64;

    pub fn new() -> Self {
        Self { items: Vec::with_capacity(Self::CAPACITY) }
    }

    pub fn try_push(&mut self, command: Command) -> Result<(), Command> {
        if self.items.len() >= Self::CAPACITY {
            return Err(command);
        }
        self.items.push(command);
        Ok(())
    }

    // moves every queued command into `out`, oldest first
    pub fn drain(&mut self, out: &mut Vec<Command>) {
        out.append(&mut self.items);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}
