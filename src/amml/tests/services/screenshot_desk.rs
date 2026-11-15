use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use alaydriem_headless_input::services::{HeadlessLink, ScreenshotDesk};
use amm_abi::types::{AmmApi, AmmScreenshotLayers, AmmStatus};
use amm_client::test_support::FakeTable;
use amm_core::structs::capture::ScreenshotLayers;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amm_core::structs::lease::AcquireOutcome;
use amm_core::structs::registry::ModIndex;
use headless_input_core::structs::{ScreenshotLayer, ScreenshotRequest};
use headless_input_mcp::port::PortError;

// a desk, a link on the fake manager and threads that play the port; the test thread plays the game thread
struct Rig {
    desk: Arc<ScreenshotDesk>,
    link: HeadlessLink,
}

impl Rig {
    const SHORT: Duration = Duration::from_millis(200);
    const WAIT: Duration = Duration::from_secs(5);

    fn new(timeout: Duration) -> Self {
        let link = HeadlessLink::new(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR)).unwrap();
        Self { desk: ScreenshotDesk::new_shared(timeout), link }
    }

    fn request(layer: ScreenshotLayer) -> ScreenshotRequest {
        ScreenshotRequest { layer, max_width: ScreenshotRequest::DEFAULT_MAX_WIDTH }
    }

    fn ask(&self, layer: ScreenshotLayer, linked: bool) -> JoinHandle<Result<PathBuf, PortError>> {
        let desk = Arc::clone(&self.desk);
        std::thread::spawn(move || desk.take(Self::request(layer), PathBuf::from("shot.png"), linked))
    }

    // pumps as a game frame would until the fake manager has seen `count` requests
    fn pump_until_requested(&self, count: usize) {
        let deadline = Instant::now() + Self::WAIT;
        while FakeTable::with(|state| state.screenshot_requests.len()) < count {
            assert!(Instant::now() < deadline, "the request did not reach the manager");
            self.desk.pump(&self.link);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn event(&self, event: ModEvent) {
        self.desk.on_event(&event);
    }
}

#[test]
fn request_becomes_one_manager_call_with_the_path_and_layer() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 1);
    let rig = Rig::new(Rig::WAIT);
    let game = rig.ask(ScreenshotLayer::Game, true);
    rig.pump_until_requested(1);
    rig.event(ModEvent::ScreenshotWritten { sequence: 1, ok: true });
    game.join().unwrap().unwrap();
    let mods = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(2);
    rig.event(ModEvent::ScreenshotWritten { sequence: 1, ok: true });
    mods.join().unwrap().unwrap();

    let layers = FakeTable::with(|state| state.screenshot_layers.clone());
    let paths = FakeTable::with(|state| state.screenshot_requests.clone());
    assert_eq!(
        layers,
        vec![
            AmmScreenshotLayers::from(ScreenshotLayers::GameOnly) as u8,
            AmmScreenshotLayers::from(ScreenshotLayers::WithModLayers) as u8,
        ]
    );
    assert_eq!(paths, vec!["shot.png".to_string(), "shot.png".to_string()]);
}

#[test]
fn written_for_the_right_sequence_answers_the_caller() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 7);
    let rig = Rig::new(Rig::WAIT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.event(ModEvent::ScreenshotTaken { sequence: 7, ok: true });
    rig.event(ModEvent::ScreenshotWritten { sequence: 7, ok: true });

    assert_eq!(asking.join().unwrap(), Ok(PathBuf::from("shot.png")));
}

#[test]
fn written_for_another_sequence_does_not() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 7);
    let rig = Rig::new(Rig::SHORT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.event(ModEvent::ScreenshotWritten { sequence: 8, ok: true });

    assert!(matches!(asking.join().unwrap(), Err(PortError::Timeout(_))));
}

#[test]
fn taken_not_ok_answers_an_error() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 3);
    let rig = Rig::new(Rig::WAIT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.event(ModEvent::ScreenshotTaken { sequence: 3, ok: false });

    assert!(matches!(asking.join().unwrap(), Err(PortError::Refused(_))));
}

#[test]
fn unsupported_manager_answers_an_error_at_once() {
    let api = AmmApi { request_screenshot: None, request_screenshot_layers: None, ..FakeTable::api() };
    let link = HeadlessLink::from_link(FakeTable::link_without_frame_on(api)).unwrap();
    let desk = ScreenshotDesk::new_shared(Rig::WAIT);
    let port_desk = Arc::clone(&desk);
    let started = Instant::now();
    let asking = std::thread::spawn(move || {
        port_desk.take(Rig::request(ScreenshotLayer::Mods), PathBuf::from("shot.png"), true)
    });
    while !asking.is_finished() {
        assert!(started.elapsed() < Rig::WAIT, "the desk did not answer");
        desk.pump(&link);
        std::thread::sleep(Duration::from_millis(1));
    }

    let result = asking.join().unwrap();

    assert!(matches!(result, Err(PortError::Refused(_))), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(FakeTable::with(|state| state.screenshot_requests.is_empty()));
}

#[test]
fn not_linked_answers_an_error_at_once() {
    let rig = Rig::new(Rig::WAIT);
    let started = Instant::now();

    let result = rig.desk.take(Rig::request(ScreenshotLayer::Mods), PathBuf::from("shot.png"), false);

    assert_eq!(result, Err(PortError::Refused("the mod manager is not linked".to_string())));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn no_answer_times_out_then_the_next_request_is_accepted() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 5);
    let rig = Rig::new(Rig::SHORT);
    let first = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);
    let timed_out = first.join().unwrap();

    let second = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(2);
    rig.event(ModEvent::ScreenshotWritten { sequence: 5, ok: true });

    assert!(matches!(timed_out, Err(PortError::Timeout(_))), "{timed_out:?}");
    assert_eq!(second.join().unwrap(), Ok(PathBuf::from("shot.png")));
}

#[test]
fn second_request_while_one_is_in_flight_is_refused() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 9);
    let rig = Rig::new(Rig::WAIT);
    let first = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    let second = rig.desk.take(Rig::request(ScreenshotLayer::Mods), PathBuf::from("other.png"), true);
    rig.event(ModEvent::ScreenshotWritten { sequence: 9, ok: true });

    assert!(matches!(second, Err(PortError::Refused(_))), "{second:?}");
    assert_eq!(first.join().unwrap(), Ok(PathBuf::from("shot.png")));
}

fn acquired() -> Vec<Capability> {
    FakeTable::with(|state| state.acquired.clone())
}

fn released() -> Vec<Capability> {
    FakeTable::with(|state| state.released.clone())
}

#[test]
fn the_overlay_lease_is_held_for_the_shot_and_released_after_written() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 2);
    let rig = Rig::new(Rig::WAIT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    assert_eq!(acquired(), vec![Capability::Overlay]);
    assert!(released().is_empty());

    rig.event(ModEvent::ScreenshotWritten { sequence: 2, ok: true });
    rig.desk.pump(&rig.link);

    assert_eq!(asking.join().unwrap(), Ok(PathBuf::from("shot.png")));
    assert_eq!(released(), vec![Capability::Overlay]);
}

#[test]
fn the_overlay_lease_is_released_after_a_failed_shot() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 3);
    let rig = Rig::new(Rig::WAIT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.event(ModEvent::ScreenshotTaken { sequence: 3, ok: false });
    rig.desk.pump(&rig.link);

    assert!(asking.join().unwrap().is_err());
    assert_eq!(released(), vec![Capability::Overlay]);
}

#[test]
fn the_overlay_lease_is_released_when_the_manager_refuses_the_request() {
    FakeTable::set_screenshot_answer(AmmStatus::NotPermitted, 0);
    let rig = Rig::new(Rig::WAIT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    assert!(matches!(asking.join().unwrap(), Err(PortError::Refused(_))));
    assert_eq!(acquired(), vec![Capability::Overlay]);
    assert_eq!(released(), vec![Capability::Overlay]);
}

#[test]
fn the_overlay_lease_is_released_on_the_next_pump_after_an_abandoned_shot() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 4);
    let rig = Rig::new(Rig::SHORT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);
    let result = asking.join().unwrap();
    assert!(matches!(result, Err(PortError::Timeout(_))), "{result:?}");
    assert!(released().is_empty());

    rig.desk.pump(&rig.link);

    assert_eq!(released(), vec![Capability::Overlay]);
}

#[test]
fn a_denied_overlay_lease_answers_an_error_at_once_with_no_request() {
    FakeTable::set_acquire_outcome(Capability::Overlay, AcquireOutcome::Denied { holder: ModIndex(2) });
    let rig = Rig::new(Rig::WAIT);
    let started = Instant::now();
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    while !asking.is_finished() {
        assert!(started.elapsed() < Rig::WAIT, "the desk did not answer");
        rig.desk.pump(&rig.link);
        std::thread::sleep(Duration::from_millis(1));
    }

    let result = asking.join().unwrap();

    assert_eq!(result, Err(PortError::Refused("the manager refused the Overlay lease".to_string())));
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(FakeTable::with(|state| state.screenshot_requests.is_empty()));
    assert!(released().is_empty());
}

#[test]
fn close_answers_a_waiting_caller_at_once_and_refuses_later_ones() {
    let rig = Rig::new(Rig::WAIT);
    let started = Instant::now();
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    std::thread::sleep(Duration::from_millis(50));

    rig.desk.close();
    let waiting = asking.join().unwrap();
    let later = rig.desk.take(Rig::request(ScreenshotLayer::Mods), PathBuf::from("shot.png"), true);

    assert!(matches!(waiting, Err(PortError::Refused(_))), "{waiting:?}");
    assert!(matches!(later, Err(PortError::Refused(_))), "{later:?}");
    assert!(started.elapsed() < Duration::from_secs(1));

    rig.desk.open();
    let reopened = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);
    rig.event(ModEvent::ScreenshotWritten { sequence: 1, ok: true });
    assert_eq!(reopened.join().unwrap(), Ok(PathBuf::from("shot.png")));
}

#[test]
fn close_gives_the_lease_of_a_shot_in_flight_back_on_the_next_pump() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, 6);
    let rig = Rig::new(Rig::WAIT);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.desk.close();
    rig.desk.pump(&rig.link);

    assert!(asking.join().unwrap().is_err());
    assert_eq!(released(), vec![Capability::Overlay]);
}

#[test]
fn an_unsupported_manager_answers_at_once_without_a_pump() {
    let rig = Rig::new(Rig::WAIT);
    rig.desk.set_supported(false);
    let started = Instant::now();

    let result = rig.desk.take(Rig::request(ScreenshotLayer::Mods), PathBuf::from("shot.png"), true);

    assert!(matches!(result, Err(PortError::Refused(_))), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(1));
}
