use std::sync::Arc;

use headless_input_core::control::{Step, Ticks};
use headless_input_core::player::KeySet;
use headless_input_core::run::LiveAction;
use headless_input_core::structs::Capabilities;

fn join() -> LiveAction {
    LiveAction::JoinServer {
        host: "127.0.0.1".to_string(),
        address: std::net::Ipv4Addr::LOCALHOST,
        port: 19132,
        timeout: None,
    }
}

#[test]
fn a_live_action_names_the_first_capability_it_lacks() {
    let none = Capabilities::new(false, false, false, false, false, false);
    let turn = LiveAction::Turn { yaw: 90.0, pitch: 0.0, ticks: Ticks(10) };
    let look = LiveAction::LookAt { yaw: 90.0, pitch: 0.0, ticks: Ticks(10) };

    assert_eq!(none.missing_for_live(&turn), Some("the player rotation"));
    assert_eq!(none.missing_for_live(&look), Some("the player rotation"));
    assert_eq!(none.missing_for_live(&join()), Some("the server join"));
    assert_eq!(none.missing_for_live(&LiveAction::LeaveWorld { timeout: None }), Some("the world leave"));
    assert_eq!(none.missing_for_live(&LiveAction::Hold { keys: KeySet::new(), ticks: None }), None);
}

#[test]
fn a_command_step_names_the_command_capability_it_lacks() {
    let step = Step::Command { text: Arc::from("/tp @s ~ ~ ~") };

    assert_eq!(Capabilities::new(true, true, true, true, true, false).missing_for(&step), Some("sending a command"));
    assert_eq!(Capabilities::all().missing_for(&step), None);
}

#[test]
fn a_live_join_needs_the_leave_too() {
    let join_only = Capabilities::new(true, true, true, true, false, true);

    assert_eq!(join_only.missing_for_live(&join()), Some("the world leave"));
    assert_eq!(Capabilities::all().missing_for_live(&join()), None);
}
