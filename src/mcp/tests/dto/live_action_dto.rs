//! The press action: a key name becomes a live press, or a `Bad` error that says why not.

use headless_input_core::run::LiveAction;
use headless_input_mcp::dto::LiveActionDto;
use headless_input_mcp::port::PortError;

fn press(key: &str) -> Result<LiveAction, PortError> {
    LiveActionDto::Press { key: key.to_owned() }.action(|_| None, |_| None)
}

#[test]
fn press_turns_a_key_name_into_a_live_press() {
    match press("Esc") {
        Ok(LiveAction::Press { key }) => assert_eq!(key.vk(), 0x1B),
        other => panic!("expected a live press, got {other:?}"),
    }
}

#[test]
fn press_refuses_unknown_and_owned_keys_as_bad() {
    match press("Banana") {
        Err(PortError::Bad(text)) => {
            assert_eq!(text, r#"unknown key "Banana" (use a key name such as "E", "Esc" or "F5")"#)
        }
        other => panic!("expected Bad, got {other:?}"),
    }
    match press("Space") {
        Err(PortError::Bad(text)) => assert_eq!(text, r#"key "Space" is driven by the jump tool"#),
        other => panic!("expected Bad, got {other:?}"),
    }
}
