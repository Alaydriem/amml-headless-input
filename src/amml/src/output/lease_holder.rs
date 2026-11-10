use crate::errors::LeaseError;
use crate::traits::ManagerPort;

// Whether the mod holds the `Input` lease. A run or live session starts only after `begin` succeeds, and every end
// releases the held input before `end` gives the lease back (`InputOutput::end`)
pub struct LeaseHolder {
    held: bool,
}

impl LeaseHolder {
    pub fn new() -> Self {
        Self { held: false }
    }

    // asks the manager for the lease; a held lease is not asked for again
    pub fn begin(&mut self, port: &impl ManagerPort) -> Result<(), LeaseError> {
        if !self.held {
            port.acquire_input()?;
            self.held = true;
        }
        Ok(())
    }

    // gives the lease back; nothing when it is not held
    pub fn end(&mut self, port: &impl ManagerPort) {
        if self.held {
            port.release_input();
            self.held = false;
        }
    }

    pub fn is_held(&self) -> bool {
        self.held
    }
}

impl Default for LeaseHolder {
    fn default() -> Self {
        Self::new()
    }
}
