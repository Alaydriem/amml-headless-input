//! The member executor: world names resolve in the instance that runs the command, and only when one is asked for.

use base64::Engine;
use headless_input_core::structs::{ScreenshotLayer, ScreenshotShot, WorldEntry, WorldStorage};
use headless_input_mcp::PortError;
use headless_input_mcp::dto::{LiveActionDto, MemberReply, MemberRequest};
use headless_input_mcp::hub::MemberExecutor;
use headless_input_mcp::port::HostResolver;

use crate::support::FakePort;

struct Worlds;

impl Worlds {
    fn entry(level_id: &str, name: &str, storage: WorldStorage, last_played: Option<u64>) -> WorldEntry {
        WorldEntry {
            level_id: level_id.to_owned(),
            name: name.to_owned(),
            storage,
            owner: "owner".to_owned(),
            last_played,
        }
    }

    fn port() -> FakePort {
        FakePort::with_worlds(vec![Self::entry("id1", "Test Flat", WorldStorage::User, None)])
    }

    fn join(world: &str) -> MemberRequest {
        MemberRequest::Live(LiveActionDto::JoinWorld { world: world.to_owned(), timeout: None })
    }
}

#[test]
fn member_executor_join_world_resolves_in_the_instance() {
    let port = Worlds::port();
    let reply = MemberExecutor::execute(&port, &HostResolver::new(), Worlds::join("test flat"));
    assert_eq!(reply, MemberReply::Done);
    let calls = port.calls();
    assert!(calls.iter().any(|call| call.starts_with("live JoinWorld") && call.contains("\"id1\"")), "{calls:?}");
}

#[test]
fn member_executor_unknown_world_fails_at_once() {
    let port = Worlds::port();
    let reply = MemberExecutor::execute(&port, &HostResolver::new(), Worlds::join("Nope"));
    assert_eq!(reply, MemberReply::Failed(PortError::UnknownWorld { world: "Nope".to_owned() }));
    assert!(!port.calls().iter().any(|call| call.starts_with("live")), "{:?}", port.calls());
}

#[test]
fn member_executor_blank_world_fails_before_any_lookup() {
    for blank in ["", "   "] {
        let port = Worlds::port();
        let reply = MemberExecutor::execute(&port, &HostResolver::new(), Worlds::join(blank));
        assert_eq!(reply, MemberReply::Failed(PortError::Bad("world is required".to_owned())));
        assert!(!port.calls().iter().any(|call| call == "worlds" || call.starts_with("live")), "{:?}", port.calls());
    }
}

#[test]
fn member_executor_reads_worlds_only_when_needed() {
    let port = Worlds::port();
    let reply =
        MemberExecutor::execute(&port, &HostResolver::new(), MemberRequest::Live(LiveActionDto::Hotbar { slot: 2 }));
    assert_eq!(reply, MemberReply::Done);
    assert!(!port.calls().contains(&"worlds".to_owned()), "{:?}", port.calls());
}

#[test]
fn member_executor_control_file_join_world_uses_the_instance_catalog() {
    let port = Worlds::port();
    let file = "name = \"j\"\n\nstep \"join_world\" {\n  world = \"Nope\"\n}\n";
    let reply = MemberExecutor::execute(&port, &HostResolver::new(), MemberRequest::RunControlFile(file.to_owned()));
    match reply {
        MemberReply::Failed(PortError::Bad(text)) => assert!(text.contains("no world named \"Nope\""), "{text}"),
        other => panic!("{other:?}"),
    }
    let known = "name = \"j\"\n\nstep \"join_world\" {\n  world = \"test flat\"\n}\n";
    let reply = MemberExecutor::execute(&port, &HostResolver::new(), MemberRequest::RunControlFile(known.to_owned()));
    assert!(matches!(reply, MemberReply::Run(_)), "{reply:?}");
}

#[test]
fn list_worlds_answers_in_search_order() {
    let port = FakePort::with_worlds(vec![
        Worlds::entry("s1", "Shared One", WorldStorage::Shared, None),
        Worlds::entry("u1", "User One", WorldStorage::User, Some(1_784_137_645)),
    ]);
    let reply = MemberExecutor::execute(&port, &HostResolver::new(), MemberRequest::ListWorlds);
    let MemberReply::Worlds(worlds) = reply else { panic!("{reply:?}") };
    let listed: Vec<(&str, &str, Option<&str>)> =
        worlds.iter().map(|w| (w.level_id.as_str(), w.storage.as_str(), w.last_played.as_deref())).collect();
    assert_eq!(listed, [("u1", "user", Some("2026-07-15T17:47:25Z")), ("s1", "shared", None)]);
}

struct Shots;

impl Shots {
    fn shot() -> ScreenshotShot {
        ScreenshotShot {
            png: vec![0x89, 0x50],
            path: std::path::PathBuf::from("shots/one.png"),
            width: 640,
            height: 360,
            source_width: 1920,
            source_height: 1080,
            layer: ScreenshotLayer::Game,
        }
    }
}

#[test]
fn screenshot_bad_arguments_fail_before_the_port() {
    let port = FakePort::with_screenshot(Shots::shot());
    for request in [
        MemberRequest::Screenshot { layers: Some("ui".to_owned()), max_width: None },
        MemberRequest::Screenshot { layers: None, max_width: Some(10) },
    ] {
        let reply = MemberExecutor::execute(&port, &HostResolver::new(), request);
        assert!(matches!(reply, MemberReply::Failed(PortError::Bad(_))), "{reply:?}");
    }
    assert!(!port.calls().contains(&"screenshot".to_owned()), "{:?}", port.calls());
}

#[test]
fn screenshot_reply_carries_the_image() {
    let port = FakePort::with_screenshot(Shots::shot());
    let reply = MemberExecutor::execute(
        &port,
        &HostResolver::new(),
        MemberRequest::Screenshot { layers: Some("game".to_owned()), max_width: Some(640) },
    );
    let MemberReply::Screenshot(dto) = reply else { panic!("{reply:?}") };
    let bytes = base64::engine::general_purpose::STANDARD.decode(&dto.png_base64).unwrap();
    assert_eq!(bytes, vec![0x89, 0x50]);
    assert_eq!((dto.width, dto.height, dto.source_width, dto.source_height), (640, 360, 1920, 1080));
    assert_eq!(dto.layers, "game");
    assert_eq!(dto.path, "shots/one.png");
}
