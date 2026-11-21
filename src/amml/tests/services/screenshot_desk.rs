use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use alaydriem_headless_input::errors::OutputError;
use alaydriem_headless_input::services::ScreenshotDesk;
use alaydriem_headless_input::traits::ScreenshotPort;
use amm_core::structs::event::ModEvent;
use amml_api::ShotId;
use headless_input_core::structs::{ScreenshotLayer, ScreenshotRequest};
use headless_input_mcp::port::PortError;

// a loader stand-in: records each request, answers it with the set sequence or refuses it
struct FakeShots {
    requests: RefCell<Vec<(PathBuf, ScreenshotLayer)>>,
    answer: Cell<Option<u32>>,
    supported: bool,
}

impl ScreenshotPort for FakeShots {
    fn supports_screenshots(&self) -> bool {
        self.supported
    }

    fn request_screenshot(&self, path: &Path, layer: ScreenshotLayer) -> Result<u32, OutputError> {
        self.requests.borrow_mut().push((path.to_path_buf(), layer));
        self.answer.get().ok_or(OutputError::Refused)
    }
}

// a desk, a loader stand-in and threads that play the port; the test thread plays the game thread
struct Rig {
    desk: Arc<ScreenshotDesk>,
    port: FakeShots,
}

impl Rig {
    const SHORT: Duration = Duration::from_millis(200);
    const WAIT: Duration = Duration::from_secs(5);

    fn new(timeout: Duration) -> Self {
        let port = FakeShots { requests: RefCell::new(Vec::new()), answer: Cell::new(Some(1)), supported: true };
        Self { desk: ScreenshotDesk::new_shared(timeout), port }
    }

    // the sequence the loader gives the next requests
    fn answer(&self, sequence: u32) {
        self.port.answer.set(Some(sequence));
    }

    fn request(layer: ScreenshotLayer) -> ScreenshotRequest {
        ScreenshotRequest { layer, max_width: ScreenshotRequest::DEFAULT_MAX_WIDTH }
    }

    fn ask(&self, layer: ScreenshotLayer, linked: bool) -> JoinHandle<Result<PathBuf, PortError>> {
        let desk = Arc::clone(&self.desk);
        std::thread::spawn(move || desk.take(Self::request(layer), PathBuf::from("shot.png"), linked))
    }

    // pumps as a game frame would until the loader stand-in has seen `count` requests
    fn pump_until_requested(&self, count: usize) {
        let deadline = Instant::now() + Self::WAIT;
        while self.port.requests.borrow().len() < count {
            assert!(Instant::now() < deadline, "the request did not reach the loader");
            self.desk.pump(&self.port);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn event(&self, event: ModEvent) {
        self.desk.on_event(&event);
    }
}

#[test]
fn request_becomes_one_manager_call_with_the_path_and_layer() {
    let rig = Rig::new(Rig::WAIT);
    rig.answer(1);
    let game = rig.ask(ScreenshotLayer::Game, true);
    rig.pump_until_requested(1);
    rig.event(ModEvent::ScreenshotWritten { id: ShotId(1), ok: true });
    game.join().unwrap().unwrap();
    let mods = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(2);
    rig.event(ModEvent::ScreenshotWritten { id: ShotId(1), ok: true });
    mods.join().unwrap().unwrap();

    let requests = rig.port.requests.borrow().clone();
    assert_eq!(
        requests,
        vec![(PathBuf::from("shot.png"), ScreenshotLayer::Game), (PathBuf::from("shot.png"), ScreenshotLayer::Mods)]
    );
}

#[test]
fn written_for_the_right_sequence_answers_the_caller() {
    let rig = Rig::new(Rig::WAIT);
    rig.answer(7);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.event(ModEvent::ScreenshotTaken { id: ShotId(7), ok: true });
    rig.event(ModEvent::ScreenshotWritten { id: ShotId(7), ok: true });

    assert_eq!(asking.join().unwrap(), Ok(PathBuf::from("shot.png")));
}

#[test]
fn written_for_another_sequence_does_not() {
    let rig = Rig::new(Rig::SHORT);
    rig.answer(7);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.event(ModEvent::ScreenshotWritten { id: ShotId(8), ok: true });

    assert!(matches!(asking.join().unwrap(), Err(PortError::Timeout(_))));
}

#[test]
fn taken_not_ok_answers_an_error() {
    let rig = Rig::new(Rig::WAIT);
    rig.answer(3);
    let asking = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    rig.event(ModEvent::ScreenshotTaken { id: ShotId(3), ok: false });

    assert!(matches!(asking.join().unwrap(), Err(PortError::Refused(_))));
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
    let rig = Rig::new(Rig::SHORT);
    rig.answer(5);
    let first = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);
    let timed_out = first.join().unwrap();

    let second = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(2);
    rig.event(ModEvent::ScreenshotWritten { id: ShotId(5), ok: true });

    assert!(matches!(timed_out, Err(PortError::Timeout(_))), "{timed_out:?}");
    assert_eq!(second.join().unwrap(), Ok(PathBuf::from("shot.png")));
}

#[test]
fn second_request_while_one_is_in_flight_is_refused() {
    let rig = Rig::new(Rig::WAIT);
    rig.answer(9);
    let first = rig.ask(ScreenshotLayer::Mods, true);
    rig.pump_until_requested(1);

    let second = rig.desk.take(Rig::request(ScreenshotLayer::Mods), PathBuf::from("other.png"), true);
    rig.event(ModEvent::ScreenshotWritten { id: ShotId(9), ok: true });

    assert!(matches!(second, Err(PortError::Refused(_))), "{second:?}");
    assert_eq!(first.join().unwrap(), Ok(PathBuf::from("shot.png")));
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
    rig.event(ModEvent::ScreenshotWritten { id: ShotId(1), ok: true });
    assert_eq!(reopened.join().unwrap(), Ok(PathBuf::from("shot.png")));
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
