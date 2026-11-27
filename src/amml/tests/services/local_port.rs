use std::path::PathBuf;
use std::time::Duration;

use alaydriem_headless_input::services::{HeadlessDescriptor, HeadlessLink, HeadlessRuntime};
use amm_client::test_support::{FakeConnection, FakeTable};
use amm_core::structs::event::ModEvent;
use amml_api::ShotId;
use amml_api::testing::FakeGame;
use headless_input_core::control::Ticks;
use headless_input_core::run::{LiveAction, RunDefaults, RunState};
use headless_input_core::structs::{ScreenshotLayer, ScreenshotRequest, ScreenshotRetention};
use headless_input_mcp::port::{HeadlessPort, PortError};

use crate::support::{RunHarness, RuntimeDirs};

// enables `runtime` with a link connected to the fake manager and attaches it; the loader the link runs on
fn attach(runtime: &HeadlessRuntime) -> FakeGame {
    let FakeConnection { link, game } = FakeTable::link_without_frame_with(HeadlessDescriptor::TEXT);
    runtime.enable(Some(HeadlessLink::new(link, true)), true);
    runtime.follow_link();
    game
}

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
    let runtime = HeadlessRuntime::new(RuntimeDirs::folders());
    let port = runtime.local_port(RunHarness::WAIT);
    assert_eq!(port.control_folder(), PathBuf::from(RuntimeDirs::DATA_DIR).join("control"));

    FakeTable::set_config("files {\n  folder = \"D:/runs\"\n}\n");
    let _game = attach(&runtime);

    assert_eq!(port.control_folder(), PathBuf::from("D:/runs"));
}

#[test]
fn defaults_follow_the_settings() {
    let runtime = HeadlessRuntime::new(RuntimeDirs::folders());
    let port = runtime.local_port(RunHarness::WAIT);
    FakeTable::set_config("run {\n  goal_timeout = \"10s\"\n}\n");
    let _game = attach(&runtime);

    assert_eq!(port.defaults(), RunDefaults::new(&runtime.config()));
    assert_eq!(port.defaults().goal_timeout, Ticks(200));
}

#[test]
fn worlds_reads_the_instance_folders() {
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("instances").join("inst");
    let data = game.join("appdata");
    std::fs::create_dir_all(game.join("amml")).unwrap();
    std::fs::write(game.join("amml").join("instance.hcl"), "data_dir = \"appdata\"\n").unwrap();
    let level = data
        .join("Minecraft Bedrock")
        .join("Users")
        .join("123")
        .join("games")
        .join("com.mojang")
        .join("minecraftWorlds")
        .join("id1");
    std::fs::create_dir_all(&level).unwrap();
    std::fs::write(level.join("level.dat"), b"x").unwrap();
    std::fs::write(level.join("levelname.txt"), "Test Flat").unwrap();
    let mut dirs = RuntimeDirs::folders();
    dirs.instance_dir = Some(game.clone());
    let runtime = HeadlessRuntime::new(dirs);

    let catalog = runtime.local_port(RunHarness::WAIT).worlds().unwrap();

    assert_eq!(catalog.find("Test Flat").unwrap().level_id, "id1");
}

#[test]
fn screenshot_prepares_and_prunes() {
    let temp = tempfile::tempdir().unwrap();
    let mut dirs = RuntimeDirs::folders();
    dirs.data_dir = temp.path().join("data");
    let shots = dirs.data_dir.join("screenshots");
    std::fs::create_dir_all(&shots).unwrap();
    for stamp in 1..=ScreenshotRetention::MAX_SCREENSHOTS as u64 {
        std::fs::write(shots.join(ScreenshotRetention::file_name(stamp)), b"x").unwrap();
    }
    let runtime = HeadlessRuntime::new(dirs);
    let game = attach(&runtime);
    let port = runtime.local_port(RunHarness::WAIT);
    let request = ScreenshotRequest { layer: ScreenshotLayer::Mods, max_width: 1280 };
    let asking = std::thread::spawn(move || port.screenshot(request));

    let deadline = std::time::Instant::now() + RunHarness::WAIT;
    while game.screenshots().is_empty() {
        assert!(std::time::Instant::now() < deadline, "the request did not reach the loader");
        runtime.follow_link();
        std::thread::sleep(Duration::from_millis(1));
    }
    let written = game.screenshots()[0].0.clone();
    image::RgbaImage::from_pixel(2560, 1440, image::Rgba([1, 2, 3, 255])).save(&written).unwrap();
    runtime.handle_link_events(&[ModEvent::ScreenshotWritten { id: ShotId(1), ok: true }]);
    while !asking.is_finished() {
        assert!(std::time::Instant::now() < deadline, "the screenshot was not answered");
        runtime.follow_link();
        std::thread::sleep(Duration::from_millis(1));
    }

    let shot = asking.join().unwrap().unwrap();

    assert_eq!((shot.width, shot.height), (1280, 720));
    assert_eq!(shot.path, written);
    assert_eq!(written.parent().unwrap(), shots);
    assert_eq!(std::fs::read_dir(&shots).unwrap().count(), ScreenshotRetention::MAX_SCREENSHOTS);
    assert!(!shots.join(ScreenshotRetention::file_name(1)).exists());
}
