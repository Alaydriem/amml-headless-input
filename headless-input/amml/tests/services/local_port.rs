use std::path::PathBuf;
use std::time::Duration;

use alaydriem_headless_input::services::HeadlessRuntime;
use amm_client::test_support::FakeTable;
use headless_input_core::control::Ticks;
use headless_input_core::run::{LiveAction, RunDefaults, RunState};
use headless_input_mcp::port::{HeadlessPort, PortError};

use crate::support::{RunHarness, RuntimeDirs};

#[test]
fn a_live_action_the_build_lacks_is_refused_at_once() {
    let harness = RunHarness::new();
    let port = harness.port();
    let join = LiveAction::JoinServer {
        host: "127.0.0.1".to_string(),
        address: std::net::Ipv4Addr::LOCALHOST,
        port: 19132,
        timeout: None,
    };
    let turn = LiveAction::Turn { yaw: 90.0, pitch: 0.0, ticks: Ticks(10) };

    let joined = port.live(join);
    let left = port.live(LiveAction::LeaveWorld { timeout: None });
    let turned = port.live(turn);

    assert!(matches!(joined, Err(PortError::Refused(ref why)) if why.contains("the server join")), "{joined:?}");
    assert!(matches!(left, Err(PortError::Refused(ref why)) if why.contains("the world leave")), "{left:?}");
    assert!(matches!(turned, Err(PortError::Refused(ref why)) if why.contains("the player rotation")), "{turned:?}");
    assert_eq!(harness.runtime.queued(), 0);
}

#[test]
fn a_command_no_frame_takes_times_out_and_never_runs() {
    let harness = RunHarness::new();
    let port = harness.runtime.local_port(Duration::from_millis(50));

    let result = port.run_control_file(RunHarness::file(RunHarness::WALK));
    harness.frame(&RunHarness::world());

    assert!(matches!(result, Err(PortError::Timeout(_))), "{result:?}");
    assert!(harness.manager.calls().is_empty());
    assert_eq!(port.status().state, RunState::Idle);
}

#[test]
fn bare_names_resolve_against_the_files_folder() {
    let runtime = HeadlessRuntime::new(&RuntimeDirs::dirs());
    let port = runtime.local_port(RunHarness::WAIT);
    assert_eq!(port.control_folder(), PathBuf::from(RuntimeDirs::DATA_DIR).join("control"));

    FakeTable::set_config("files {\n  folder = \"D:/runs\"\n}\n");
    runtime.enable_link();
    runtime.tick_with(|| Ok(FakeTable::table()));

    assert_eq!(port.control_folder(), PathBuf::from("D:/runs"));
}

#[test]
fn defaults_follow_the_settings() {
    let runtime = HeadlessRuntime::new(&RuntimeDirs::dirs());
    let port = runtime.local_port(RunHarness::WAIT);
    FakeTable::set_config("run {\n  goal_timeout = \"10s\"\n}\n");
    runtime.enable_link();
    runtime.tick_with(|| Ok(FakeTable::table()));

    assert_eq!(port.defaults(), RunDefaults::new(&runtime.config()));
    assert_eq!(port.defaults().goal_timeout, Ticks(200));
}
