use std::sync::atomic::{AtomicU8, Ordering};

use super::HubRole;

// The role of a hub node, written by its task and read by any thread.
#[derive(Debug)]
pub struct RoleCell(AtomicU8);

impl RoleCell {
    pub fn new(role: HubRole) -> Self {
        Self(AtomicU8::new(role.code()))
    }

    pub fn get(&self) -> HubRole {
        HubRole::from_code(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, role: HubRole) {
        self.0.store(role.code(), Ordering::Relaxed);
    }
}
