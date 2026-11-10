use amm_core::structs::input::PointerButton;
use headless_input_core::control::{InputKey, PointerAction};

// The Windows virtual-key codes of the movement keys and the hotbar keys, and the mouse button of each pointer action
pub struct KeyCodes;

impl KeyCodes {
    pub const FORWARD: u32 = 0x57;
    pub const BACK: u32 = 0x53;
    pub const LEFT: u32 = 0x41;
    pub const RIGHT: u32 = 0x44;
    pub const JUMP: u32 = 0x20;
    pub const SNEAK: u32 = 0x10;
    pub const SPRINT: u32 = 0x11;
    // the 1 key; slot N is HOTBAR_1 + N - 1, up to the 9 key
    pub const HOTBAR_1: u32 = 0x31;
    pub const HOTBAR_SLOTS: u8 = 9;

    pub fn key(key: InputKey) -> u32 {
        match key {
            InputKey::Forward => Self::FORWARD,
            InputKey::Back => Self::BACK,
            InputKey::Left => Self::LEFT,
            InputKey::Right => Self::RIGHT,
            InputKey::Jump => Self::JUMP,
            InputKey::Sneak => Self::SNEAK,
            InputKey::Sprint => Self::SPRINT,
        }
    }

    // the key of hotbar slot 1 to 9; None for any other slot
    pub fn hotbar(slot: u8) -> Option<u32> {
        (1..=Self::HOTBAR_SLOTS).contains(&slot).then(|| Self::HOTBAR_1 + u32::from(slot) - 1)
    }

    pub fn button(action: PointerAction) -> PointerButton {
        match action {
            PointerAction::Attack => PointerButton::Left,
            PointerAction::Use => PointerButton::Right,
        }
    }
}
