use alaydriem_headless_input::output::KeyCodes;
use headless_input_core::control::{InputKey, TapKey};
use headless_input_core::errors::TapKeyError;

fn refused(code: u32) -> bool {
    matches!(TapKey::parse(&format!("VK_0x{code:X}")), Err(TapKeyError::Owned { .. }))
}

#[test]
fn every_movement_and_hotbar_key_code_is_refused_as_a_press() {
    for key in InputKey::ALL {
        assert!(refused(KeyCodes::key(key)), "{key:?} should be refused");
    }
    for slot in 1..=KeyCodes::HOTBAR_SLOTS {
        let code = KeyCodes::hotbar(slot).expect("slots 1 to 9 have a key");
        assert!(refused(code), "hotbar slot {slot} should be refused");
    }
}
