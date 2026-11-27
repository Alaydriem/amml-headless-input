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
    // the number keys of the hotbar stay refused: `hotbar` selects through the HUD
    for code in 0x31..=0x39 {
        assert!(refused(code), "number key {code:#x} should be refused");
    }
}

#[test]
fn hotbar_slots_1_to_9_are_hud_slots_0_to_8() {
    assert_eq!(KeyCodes::hotbar_index(1), Some(0));
    assert_eq!(KeyCodes::hotbar_index(9), Some(8));
    assert_eq!(KeyCodes::hotbar_index(0), None);
    assert_eq!(KeyCodes::hotbar_index(10), None);
}
