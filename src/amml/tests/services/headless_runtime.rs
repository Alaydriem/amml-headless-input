use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use alaydriem_headless_input::services::{HeadlessDescriptor, HeadlessLink, HeadlessRuntime};
use alaydriem_headless_input::types::{FrameView, LinkState, RuntimeFolders};
use amm_client::test_support::{FakeConnection, FakeTable};
use amm_core::structs::event::{ModEvent, ReleaseReason};
use amml_api::MoveIntent;
use amml_api::testing::FakeGame;
use headless_input_core::run::{EndReason, RunId, RunState};
use headless_input_core::structs::Capabilities;
use headless_input_mcp::port::{HeadlessPort, PortError};

use crate::support::fake_command_client::FakeCommandClient;
use crate::support::fake_join_client::FakeJoinClient;
use crate::support::fake_manager::{Call, FakeManager};
use crate::support::{RunHarness, RuntimeDirs};

fn runtime() -> HeadlessRuntime {
    HeadlessRuntime::new(RuntimeDirs::folders())
}

// enables `runtime` with a link connected to the fake manager and attaches it; the loader the link runs on
fn attach(runtime: &HeadlessRuntime) -> FakeGame {
    let FakeConnection { link, game } = FakeTable::link_without_frame_with(HeadlessDescriptor::TEXT);
    runtime.enable(Some(HeadlessLink::new(link, true)), true);
    runtime.follow_link();
    game
}

fn forward(down: bool) -> Call {
    let forward = if down { 1.0 } else { 0.0 };
    Call::Move(MoveIntent { forward, ..MoveIntent::default() })
}

const WAIT: &str = "name = \"wait\"\n\nstep \"wait\" {\n  for = \"5s\"\n}\n";

// a runtime attached to the fake manager table, driven through its own link
struct Linked {
    runtime: HeadlessRuntime,
    join: FakeJoinClient,
    command: FakeCommandClient,
    game: FakeGame,
    seen_moves: std::cell::RefCell<Vec<String>>,
}

impl Linked {
    fn new() -> Self {
        let runtime = runtime();
        let game = attach(&runtime);
        assert_eq!(runtime.link_state(), LinkState::Attached);
        Self {
            runtime,
            join: FakeJoinClient::in_world(),
            command: FakeCommandClient::new(),
            game,
            seen_moves: Default::default(),
        }
    }

    fn start(&self, text: &str) {
        let port = self.runtime.local_port(RunHarness::WAIT);
        let file = RunHarness::file(text);
        let asking = std::thread::spawn(move || port.run_control_file(file));
        let deadline = Instant::now() + RunHarness::WAIT;
        while self.runtime.queued() < 1 {
            assert!(Instant::now() < deadline, "the command did not reach the queue");
            std::thread::sleep(Duration::from_millis(1));
        }
        self.frame(&RunHarness::world());
        assert_eq!(asking.join().unwrap(), Ok(RunId(1)));
    }

    // the link events, then the frame, as one ClientTick runs them
    fn update(&self, frame: &FrameView) {
        self.runtime.follow_link();
        self.frame(frame);
    }

    fn frame(&self, frame: &FrameView) {
        self.runtime.drive_linked_frame(frame, &self.join, &self.command);
    }

    fn tick(&self) {
        self.runtime.drive_linked_tick(Some(&RunHarness::world_view()));
    }

    fn state(&self) -> RunState {
        self.runtime.local_port(RunHarness::WAIT).status().state
    }

    fn reason(&self) -> Option<EndReason> {
        self.runtime.local_port(RunHarness::WAIT).status().last.map(|last| last.reason.clone())
    }

    // whether the game holds a move after the next frame: the last move line that reached it is not the clear
    fn move_held(&self) -> bool {
        self.game.frame();
        self.game.injected().iter().rev().find(|line| line.starts_with("move")).is_some_and(|line| line != STILL)
    }

    // each move that reached the game since the last call: the loader applies queued moves at the next frame
    fn sent_moves(&self) -> Vec<String> {
        self.game.frame();
        let moves: Vec<String> = self.game.injected().into_iter().filter(|line| line.starts_with("move")).collect();
        let mut seen = self.seen_moves.borrow_mut();
        let fresh = moves[seen.len().min(moves.len())..].to_vec();
        *seen = moves;
        fresh
    }
}

// the clear of the move
const STILL: &str = "move none";

#[test]
fn the_saved_settings_are_read_on_attach_and_again_on_change() {
    FakeTable::set_config("mcp {\n  port = 29001\n}\n");
    let runtime = runtime();
    let _game = attach(&runtime);
    assert_eq!(runtime.link_state(), LinkState::Attached);
    assert_eq!(runtime.config().mcp_port, 29001);

    FakeTable::set_config("mcp {\n  port = 29002\n}\n");
    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    runtime.follow_link();

    assert_eq!(runtime.config().mcp_port, 29002);
}

#[test]
fn no_manager_link_leaves_the_defaults() {
    let runtime = runtime();
    runtime.enable(None, true);

    runtime.follow_link();

    assert_eq!(runtime.link_state(), LinkState::Off);
    assert_eq!(runtime.config().mcp_port, 28250);
}

#[test]
fn a_link_follow_before_enable_does_nothing() {
    let runtime = runtime();

    runtime.follow_link();

    assert_eq!(runtime.link_state(), LinkState::Disabled);
}

#[test]
fn detach_drops_the_link_and_stops_ticks_reaching_the_manager() {
    let runtime = runtime();
    let _game = attach(&runtime);
    runtime.detach();

    runtime.follow_link();

    assert_eq!(runtime.link_state(), LinkState::Disabled);
}

#[test]
fn capabilities_are_off_until_enable_sets_them() {
    assert_eq!(runtime().capabilities(), Capabilities::new(false, false, false, false, false, false, false));
}

#[test]
fn a_tick_without_a_local_player_does_not_advance() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(RunHarness::WALK)).unwrap();
    harness.manager.clear();

    harness.tick_without_player();
    harness.tick_without_player();

    assert!(harness.manager.calls().is_empty(), "{:?}", harness.manager.calls());
}

#[test]
fn a_tick_with_a_local_player_advances_the_run() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(RunHarness::WALK)).unwrap();
    harness.manager.clear();

    harness.tick_without_player();
    assert!(harness.manager.calls().is_empty());
    harness.tick();

    assert_eq!(harness.manager.calls(), vec![forward(true)]);
}

#[test]
fn a_queued_run_starts_on_the_next_frame() {
    let harness = RunHarness::new();
    let asking = harness.queue_run(RunHarness::file(RunHarness::WALK));
    assert_eq!(harness.port().status().state, RunState::Idle);
    assert!(harness.manager.calls().is_empty());

    harness.frame(&RunHarness::world());

    assert_eq!(asking.join().unwrap(), Ok(RunId(1)));
    assert_eq!(harness.manager.calls(), vec![Call::Acquire]);
    assert_eq!(harness.port().status().state, RunState::Running);
    harness.tick();
    assert_eq!(harness.manager.calls(), vec![Call::Acquire, forward(true)]);
}

#[test]
fn a_refused_lease_starts_nothing_and_names_the_holder() {
    let harness = RunHarness::with_manager(FakeManager::held_by("@other/mod"));

    let result = harness.start(RunHarness::file(RunHarness::WALK));

    assert_eq!(result, Err(PortError::Refused("@other/mod holds the input".to_string())));
    harness.tick();
    harness.frame(&RunHarness::world());
    assert_eq!(harness.manager.calls(), vec![Call::Acquire]);
    assert_eq!(harness.port().status().state, RunState::Idle);
}

#[test]
fn world_exit_ends_the_run_and_releases_the_lease() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(RunHarness::WALK)).unwrap();
    harness.tick();
    harness.frame(&RunHarness::world());
    harness.manager.clear();

    harness.frame(&RunHarness::title());

    assert_eq!(harness.manager.calls(), vec![forward(false), Call::Release]);
    let status = harness.port().status();
    assert_eq!(status.state, RunState::Idle);
    let reason = status.last.map(|last| last.reason.clone());
    assert!(matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the world was left"), "{reason:?}");
}

#[test]
fn a_refused_send_aborts_the_run_and_still_ends_the_lease() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(RunHarness::WALK)).unwrap();
    harness.manager.take_lease();
    harness.tick();
    harness.manager.clear();

    harness.frame(&RunHarness::world());

    assert_eq!(harness.manager.calls().last(), Some(&Call::Release));
    assert_eq!(harness.port().status().state, RunState::Idle);
}

#[test]
fn disable_releases_input_before_stopping_the_hub() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(RunHarness::WALK)).unwrap();
    harness.tick();
    harness.manager.clear();

    harness.runtime.release_then(Some(&harness.manager), || harness.manager.mark("hub stopped"));

    assert_eq!(harness.manager.calls(), vec![forward(false), Call::Release, Call::Mark("hub stopped")]);
    let status = harness.port().status();
    let reason = status.last.map(|last| last.reason.clone());
    assert!(matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the mod was disabled"), "{reason:?}");
}

#[test]
fn instance_id_is_the_instance_folder_name() {
    assert_eq!(HeadlessRuntime::instance_id(&RuntimeDirs::folders()).as_str(), RuntimeDirs::INSTANCE);

    let loose = RuntimeFolders { instance_dir: None, ..RuntimeDirs::folders() };
    let fallback = HeadlessRuntime::instance_id(&loose);
    assert!(fallback.as_str().starts_with("instance-"), "{fallback}");
}

#[test]
fn the_token_file_lives_in_the_shared_data_folder_or_in_the_instance_without_one() {
    let shared = HeadlessRuntime::new(RuntimeDirs::folders());
    assert_eq!(shared.token_folder(), PathBuf::from(RuntimeDirs::SHARED_DATA_DIR));

    let old_loader = HeadlessRuntime::new(RuntimeFolders { shared_data_dir: None, ..RuntimeDirs::folders() });
    assert_eq!(old_loader.token_folder(), PathBuf::from(RuntimeDirs::DATA_DIR));
}

#[test]
fn disable_answers_a_queued_command_at_once_and_it_never_runs() {
    let harness = RunHarness::new();
    let asking = harness.queue_run(RunHarness::file(RunHarness::WALK));
    let started = Instant::now();

    harness.runtime.detach();

    let result = asking.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(1), "{:?}", started.elapsed());
    assert!(matches!(result, Err(PortError::Refused(ref why)) if why == "the mod was disabled"), "{result:?}");
    assert_eq!(harness.runtime.queued(), 0);
    harness.frame(&RunHarness::world());
    assert!(harness.manager.calls().is_empty());
    assert_eq!(harness.port().status().state, RunState::Idle);
}

#[test]
fn disable_denies_pairing_at_once_until_the_next_enable() {
    let harness = RunHarness::new();
    harness.runtime.detach();
    let port = harness.port();
    let started = Instant::now();

    assert!(!port.approve_pairing("claude-code"));
    assert!(started.elapsed() < Duration::from_secs(1));
    let refused = port.stop();
    assert!(matches!(refused, Err(PortError::Refused(_))), "{refused:?}");

    harness.runtime.enable(None, true);
    let reopened = harness.runtime.local_port(Duration::from_millis(20)).stop();
    assert!(matches!(reopened, Err(PortError::Timeout(_))), "{reopened:?}");
}

#[test]
fn the_panel_opening_during_a_wait_stops_the_run_and_it_does_not_resume() {
    let linked = Linked::new();
    linked.start(WAIT);
    linked.tick();
    linked.update(&RunHarness::world());
    assert_eq!(linked.state(), RunState::Running);

    FakeTable::with(|state| state.panel_open = true);
    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Idle);
    assert_eq!(linked.reason(), Some(EndReason::Stopped));
    assert!(linked.sent_moves().is_empty());

    FakeTable::with(|state| state.panel_open = false);
    linked.tick();
    linked.update(&RunHarness::world());
    assert_eq!(linked.state(), RunState::Idle);
}

#[test]
fn another_mod_suspending_the_input_aborts_the_run_and_releases_keys_then_lease() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    linked.runtime.handle_link_events(&[ModEvent::InputLeaseSuspended]);

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Idle);
    let reason = linked.reason();
    assert!(
        matches!(reason, Some(EndReason::Aborted { ref why }) if why == "another mod took the input"),
        "{reason:?}"
    );
    assert!(!linked.move_held(), "the move is released at the end");
}

#[test]
fn another_mod_revoking_the_input_aborts_the_run() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    linked.runtime.handle_link_events(&[ModEvent::InputLeaseLost]);

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Idle);
    let reason = linked.reason();
    assert!(
        matches!(reason, Some(EndReason::Aborted { ref why }) if why == "another mod took the input"),
        "{reason:?}"
    );
    assert!(!linked.move_held(), "the move is released at the end");
}

#[test]
fn the_manager_disabling_the_mod_aborts_the_run_and_releases_it() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    linked.runtime.handle_link_events(&[ModEvent::Disabled]);

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Idle);
    let reason = linked.reason();
    assert!(matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the mod was disabled"), "{reason:?}");
    assert!(!linked.move_held(), "the move is released at the end");
}

#[test]
fn a_suspension_of_another_capability_leaves_the_run() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    linked.runtime.handle_link_events(&[ModEvent::InputLeaseResumed]);

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Running);
}

#[test]
fn fake_keys_released_drops_the_record_and_a_world_exit_on_that_frame_aborts() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    linked.runtime.handle_link_events(&[ModEvent::FakeKeysReleased { reason: ReleaseReason::WorldLeft }]);
    linked.update(&RunHarness::world());
    assert_eq!(linked.state(), RunState::Running, "a release of the fake keys is not the panel");
    linked.tick();
    assert!(linked.move_held(), "the forgotten move is sent again");

    linked.runtime.handle_link_events(&[ModEvent::FakeKeysReleased { reason: ReleaseReason::WorldLeft }]);
    linked.update(&RunHarness::title());

    let reason = linked.reason();
    assert!(matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the world was left"), "{reason:?}");
}

const ALLOW_ON: &str = "mcp {
  allow_pairing = true
}
";

// asks for a pairing on another thread, as the hub does, and returns once the question waits
fn ask_pairing(runtime: &HeadlessRuntime) -> JoinHandle<bool> {
    let port = runtime.local_port(RunHarness::WAIT);
    let asking = std::thread::spawn(move || port.approve_pairing("claude-code"));
    let deadline = Instant::now() + RunHarness::WAIT;
    while !runtime.pairing_waiting() {
        assert!(Instant::now() < deadline, "the question did not wait");
        std::thread::sleep(Duration::from_millis(1));
    }
    asking
}

fn turn_pairing_on(linked: &Linked) {
    FakeTable::set_config(ALLOW_ON);
    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    linked.runtime.follow_link();
}

fn pairing_resets() -> Vec<(String, String)> {
    FakeTable::with(|state| state.settings.iter().map(|call| (call.path.clone(), call.expression.clone())).collect())
}

#[test]
fn pairing_is_allowed_when_the_setting_turns_on_while_a_request_waits() {
    let linked = Linked::new();
    let asking = ask_pairing(&linked.runtime);

    turn_pairing_on(&linked);

    assert!(asking.join().unwrap());
}

#[test]
fn the_setting_is_turned_off_after_the_answer() {
    let linked = Linked::new();
    let asking = ask_pairing(&linked.runtime);

    turn_pairing_on(&linked);

    assert!(asking.join().unwrap());
    assert_eq!(pairing_resets(), vec![("mcp.allow_pairing".to_string(), "false".to_string())]);
    assert!(!linked.runtime.pairing_waiting());
}

#[test]
fn the_setting_turned_on_with_no_request_is_turned_off_and_allows_nothing() {
    let linked = Linked::new();

    turn_pairing_on(&linked);

    assert_eq!(pairing_resets(), vec![("mcp.allow_pairing".to_string(), "false".to_string())]);
    let asking = ask_pairing(&linked.runtime);
    linked.runtime.follow_link();
    std::thread::sleep(Duration::from_millis(50));
    assert!(!asking.is_finished(), "an earlier switch answered a later question");
    linked.runtime.detach();
    assert!(!asking.join().unwrap());
}

#[test]
fn the_setting_can_allow_a_second_request_after_the_first() {
    let linked = Linked::new();
    let first = ask_pairing(&linked.runtime);
    turn_pairing_on(&linked);
    assert!(first.join().unwrap());

    FakeTable::set_config("");
    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    linked.runtime.follow_link();
    let second = ask_pairing(&linked.runtime);
    turn_pairing_on(&linked);

    assert!(second.join().unwrap());
}

#[test]
fn a_switch_left_on_does_not_allow_the_next_request() {
    let linked = Linked::new();
    // no request waits: the switch is reset, and the fake manager keeps the value on, as when the reset is lost
    turn_pairing_on(&linked);
    assert_eq!(pairing_resets().len(), 1);
    let asking = ask_pairing(&linked.runtime);

    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    linked.runtime.follow_link();
    std::thread::sleep(Duration::from_millis(50));

    assert!(!asking.is_finished(), "a switch that was already on allowed a later request");
    linked.runtime.detach();
    assert!(!asking.join().unwrap());
}

#[test]
fn no_menu_tile_is_registered() {
    let linked = Linked::new();
    let asking = ask_pairing(&linked.runtime);

    for _ in 0..3 {
        linked.runtime.follow_link();
    }

    assert!(FakeTable::with(|state| state.menu_entries.is_empty()));
    linked.runtime.detach();
    assert!(!asking.join().unwrap());
}

const COMMAND_RUN: &str = "name = \"tp\"\n\nstep \"command\" {\n  text = \"/tp @s 481.5 81 408.5 90 0\"\n}\n\nstep \"wait\" {\n  for = \"5s\"\n}\n";

#[test]
fn a_command_step_sends_its_command_once_on_the_frame_after_its_tick() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(COMMAND_RUN)).unwrap();
    assert!(harness.command.sent().is_empty());

    harness.tick();
    assert!(harness.command.sent().is_empty());
    harness.frame(&RunHarness::world());
    harness.frame(&RunHarness::world());
    harness.tick();
    harness.tick();
    harness.frame(&RunHarness::world());

    assert_eq!(harness.command.sent(), vec!["/tp @s 481.5 81 408.5 90 0".to_string()]);
    assert_eq!(harness.port().status().state, RunState::Running);
}

#[test]
fn a_command_step_with_no_local_player_ends_the_run_and_sends_nothing() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(COMMAND_RUN)).unwrap();
    harness.command.set_player(false);

    harness.tick();
    harness.frame(&RunHarness::world());

    assert!(harness.command.sent().is_empty());
    let status = harness.port().status();
    assert_eq!(status.state, RunState::Idle);
    let reason = status.last.map(|last| last.reason.clone());
    assert!(
        matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the command could not be sent: there is no local player"),
        "{reason:?}"
    );
    assert_eq!(harness.manager.calls().last(), Some(&Call::Release));
}

#[test]
fn a_command_the_game_did_not_take_ends_the_run() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(COMMAND_RUN)).unwrap();
    harness.command.set_takes(false);

    harness.tick();
    harness.frame(&RunHarness::world());

    assert_eq!(harness.port().status().state, RunState::Idle);
    let reason = harness.port().status().last.map(|last| last.reason.clone());
    assert!(
        matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the command could not be sent: the game did not take it"),
        "{reason:?}"
    );
}

#[test]
fn a_stop_between_the_tick_and_the_frame_sends_no_command() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(COMMAND_RUN)).unwrap();
    harness.tick();
    let port = harness.port();
    let stopping = std::thread::spawn(move || port.stop());
    harness.wait_queued(1);

    harness.frame(&RunHarness::world());

    assert_eq!(stopping.join().unwrap(), Ok(()));
    assert!(harness.command.sent().is_empty());
    assert_eq!(harness.port().status().state, RunState::Idle);
}

#[test]
fn a_failed_send_left_from_an_old_run_does_not_abort_the_next_run() {
    let harness = RunHarness::new();
    harness.start(RunHarness::file(COMMAND_RUN)).unwrap();
    harness.frame(&RunHarness::world());
    harness.tick();
    harness.command.set_takes(false);
    let asking = harness.queue_run(RunHarness::file(RunHarness::WALK));

    // the world is left on this frame: the first run ends and the queued second run starts
    harness.frame(&RunHarness::title());

    assert_eq!(asking.join().unwrap(), Ok(RunId(2)));
    let status = harness.port().status();
    assert_eq!(status.state, RunState::Running);
    assert_eq!(status.run, Some(RunId(2)));
    assert!(harness.command.sent().is_empty());
}

const JOIN_RUN: &str = "name = \"join\"

step \"join_server\" {
  address = \"127.0.0.1\"
}
";

// a run with a join step whose join the fake game took, ready for the game's answer
fn joining_harness() -> RunHarness {
    let mut harness = RunHarness::new();
    harness.join = FakeJoinClient::new("start_screen");
    harness.join.set_tab_opens(false);
    harness.start(RunHarness::file(JOIN_RUN)).unwrap();
    harness.frame(&RunHarness::title());
    assert_eq!(harness.join.joins().len(), 1);
    harness
}

#[test]
fn a_failure_code_from_the_game_ends_the_join_step_failed_with_the_code() {
    let harness = joining_harness();
    harness.join.answer_with(8);

    harness.frame(&RunHarness::title());

    let status = harness.port().status();
    assert_eq!(status.state, RunState::Idle);
    let reason = status.last.map(|last| last.reason.clone());
    assert!(
        matches!(reason, Some(EndReason::Failed { ref why, .. }) if why.contains("could not resolve") && why.contains('8')),
        "{reason:?}"
    );
    assert_eq!(harness.manager.calls().last(), Some(&Call::Release));
}

#[test]
fn a_code_that_is_no_failure_leaves_the_join_running() {
    let harness = joining_harness();
    harness.join.answer_with(0);

    harness.frame(&RunHarness::title());

    assert_eq!(harness.port().status().state, RunState::Running);
}
