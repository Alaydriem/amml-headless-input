use amm_client::link::PointerOp;
use amml_api::MoveIntent;

use crate::errors::{LeaseError, OutputError};

// what the output side needs from the mod manager and the game: the `Input` lease, fake keys, pointer actions, the move,
// the hotbar select, and whether the manager panel is open. Game thread
pub trait ManagerPort {
    // a lease taken from another mod counts as granted
    fn acquire_input(&self) -> Result<(), LeaseError>;

    fn release_input(&self);

    fn send_key(&self, vk: u32, down: bool) -> Result<(), OutputError>;

    fn send_pointer(&self, op: PointerOp) -> Result<(), OutputError>;

    // holds `intent` until the next call; `MoveIntent::default()` clears it and is always taken. `Unsupported` from a
    // loader without the move area
    fn send_move(&self, intent: MoveIntent) -> Result<(), OutputError>;

    // selects HUD hotbar slot `index` (0 to 8) through the game's HUD
    fn select_hotbar(&self, index: u8) -> Result<(), OutputError>;

    // true while the manager panel is open; the manager then refuses every send
    fn panel_open(&self) -> bool;
}
