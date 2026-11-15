use headless_input_core::control::TapKey;
use headless_input_core::errors::TapKeyError;

fn vk(name: &str) -> u32 {
    TapKey::parse(name).expect("the name should resolve").vk()
}

#[test]
fn names_resolve_with_case_ignored() {
    assert_eq!(vk("e"), 0x45);
    assert_eq!(vk("E"), 0x45);
    assert_eq!(vk("f5"), 0x74);
    assert_eq!(vk("Escape"), 0x1B);
}

#[test]
fn esc_is_an_alias_of_escape() {
    assert_eq!(vk("Esc"), 0x1B);
    assert_eq!(vk("ESC"), 0x1B);
}

#[test]
fn the_vk_form_resolves() {
    assert_eq!(vk("VK_0x74"), 0x74);
}

#[test]
fn f3_is_allowed() {
    assert_eq!(vk("F3"), 0x72);
}

#[test]
fn unknown_and_blank_names_are_refused() {
    for name in ["Banana", "", "  "] {
        assert!(matches!(TapKey::parse(name), Err(TapKeyError::Unknown(_))), "{name:?} should be unknown");
    }
    let error = TapKey::parse("Banana").expect_err("unknown");
    assert_eq!(error.to_string(), r#"unknown key "Banana" (use a key name such as "E", "Esc" or "F5")"#);
}

#[test]
fn keys_owned_by_other_tools_are_refused() {
    let owned = [
        ("W", "move"),
        ("a", "move"),
        ("S", "move"),
        ("D", "move"),
        ("Space", "jump"),
        ("Shift", "sneak"),
        ("Ctrl", "sprint"),
        ("1", "hotbar"),
        ("9", "hotbar"),
        ("VK_0x57", "move"),
    ];
    for (name, wanted) in owned {
        match TapKey::parse(name) {
            Err(TapKeyError::Owned { tool, .. }) => assert_eq!(tool, wanted, "{name}"),
            other => panic!("{name} should be owned by {wanted}, got {other:?}"),
        }
    }
    let error = TapKey::parse("W").expect_err("owned");
    assert_eq!(error.to_string(), r#"key "W" is driven by the move tool"#);
}

#[test]
fn vk_codes_outside_1_to_255_are_refused() {
    for name in ["VK_0x0", "VK_0x100", "VK_0xFFFFFFFF"] {
        assert!(matches!(TapKey::parse(name), Err(TapKeyError::Unknown(_))), "{name} should be unknown");
    }
    assert_eq!(vk("VK_0xFF"), 0xFF);
}
