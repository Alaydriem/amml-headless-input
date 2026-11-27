use alaydriem_headless_input::errors::OutputError;
use alaydriem_headless_input::services::{HeadlessDescriptor, HeadlessLink};
use alaydriem_headless_input::traits::{ManagerPort, ScreenshotPort};
use amm_client::test_support::{FakeConnection, FakeTable};
use amml_api::MoveIntent;
use amml_api::testing::FakeGame;

// a connected link and the fake loader it runs on; the link is dropped before the loader
fn link(screenshots: bool) -> (FakeGame, HeadlessLink) {
    let FakeConnection { link, game } = FakeTable::link_without_frame_with(HeadlessDescriptor::TEXT);
    (game, HeadlessLink::new(link, screenshots))
}

#[test]
fn a_connected_link_reloads_a_changed_setting() {
    FakeTable::set_config("mcp {\n  enabled = false\n}\n");
    let (_game, mut link) = link(true);
    assert!(link.is_connected());
    link.reload();
    assert!(!link.config().mcp_enabled);

    FakeTable::set_config("mcp {\n  enabled = true\n}\n");
    link.reload();

    assert!(link.config().mcp_enabled);
}

#[test]
fn panel_open_is_the_manager_answer() {
    let (_game, link) = link(true);
    assert!(!link.panel_open());

    FakeTable::with(|state| state.panel_open = true);
    assert!(link.panel_open());
}

#[test]
fn a_key_without_the_lease_is_refused_and_a_code_that_is_no_key_is_unsupported() {
    let (_game, link) = link(true);

    assert_eq!(link.send_key(0x57, true), Err(OutputError::Refused));
    assert_eq!(link.send_key(0x1_0000, true), Err(OutputError::Unsupported));
    assert_eq!(link.send_key(0, true), Err(OutputError::Unsupported));
}

#[test]
fn a_held_key_and_move_reach_the_loader_under_the_lease() {
    let (game, link) = link(true);
    assert_eq!(link.acquire_input(), Ok(()));

    assert_eq!(link.send_key(0x45, true), Ok(()));
    assert_eq!(link.send_move(MoveIntent { forward: 1.0, ..MoveIntent::default() }), Ok(()));
    game.frame();

    let injected = game.injected();
    assert!(injected.contains(&"key 0x45 down".to_string()), "{injected:?}");
    assert!(injected.iter().any(|line| line.starts_with("move forward=1")), "{injected:?}");
}

#[test]
fn the_link_alone_selects_no_hotbar_slot_and_reports_the_loader_screenshot_support() {
    let (_game, link) = link(false);

    assert_eq!(link.select_hotbar(2), Err(OutputError::Unsupported));
    assert!(!link.supports_screenshots());
}
