use std::sync::Arc;

use super::{InstanceId, MemberRegistry};

// Held by one open poll, which counts in `MemberRegistry::polling`. A poll that is dropped before it finishes lost its
// connection, so the guard drops the member.
pub struct PollGuard {
    registry: Arc<MemberRegistry>,
    id: InstanceId,
    generation: u64,
    finished: bool,
}

impl PollGuard {
    pub fn new(registry: Arc<MemberRegistry>, id: InstanceId, generation: u64) -> Self {
        Self { registry, id, generation, finished: false }
    }

    pub fn finish(&mut self) {
        self.finished = true;
    }
}

impl Drop for PollGuard {
    fn drop(&mut self) {
        self.registry.poll_ended(&self.id, self.generation);
        if !self.finished {
            self.registry.connection_closed(&self.id, self.generation);
        }
    }
}
