use amm_core::structs::input::PointerOp;

use crate::errors::{LeaseError, OutputError};

// what the output side needs from the mod manager: the `Input` lease, fake keys and pointer actions, and whether the
// manager panel is open. Game thread
pub trait ManagerPort {
    // a lease taken from another mod counts as granted
    fn acquire_input(&self) -> Result<(), LeaseError>;

    fn release_input(&self);

    fn send_key(&self, vk: u32, down: bool) -> Result<(), OutputError>;

    fn send_pointer(&self, op: PointerOp) -> Result<(), OutputError>;

    // true while the manager panel is open; the manager then refuses every send
    fn panel_open(&self) -> bool;
}
