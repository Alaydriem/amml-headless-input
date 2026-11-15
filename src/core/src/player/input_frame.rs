use std::sync::Arc;

use super::{KeySet, PointerOps};
use crate::control::TapKey;
use crate::structs::JoinRequest;

// The full input the player wants on one tick. The output side sends only what changed since the last frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InputFrame {
    pub keys: KeySet,
    // (yaw, pitch) in degrees; None leaves the camera alone
    pub rotation: Option<(f32, f32)>,
    pub pointer: PointerOps,
    pub hotbar: Option<u8>,
    // the key to tap on this tick
    pub tap: Option<TapKey>,
    // shared with the join step, so a frame that repeats the request copies no text
    pub join: Option<Arc<JoinRequest>>,
    pub leave: bool,
    // the slash command to send to the server on this tick; shared with the step, so a frame copies no text
    pub command: Option<Arc<str>>,
}
