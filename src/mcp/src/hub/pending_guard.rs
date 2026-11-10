use super::Forwarder;

// Removes one command from the pending map of the forwarder when its `send` ends in any way, a dropped call too.
pub struct PendingGuard<'a> {
    forwarder: &'a Forwarder,
    command: u64,
}

impl<'a> PendingGuard<'a> {
    pub fn new(forwarder: &'a Forwarder, command: u64) -> Self {
        Self { forwarder, command }
    }
}

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        self.forwarder.forget(self.command);
    }
}
