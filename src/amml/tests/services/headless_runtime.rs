use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use alaydriem_headless_input::output::KeyCodes;
use alaydriem_headless_input::services::HeadlessRuntime;
use alaydriem_headless_input::types::{FrameView, LinkState};
use amm_client::test_support::FakeTable;
use amm_core::errors::LinkError;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amm_core::structs::registry::ModIndex;
use amml_sdk::{HookHandle, ModDirs};
use headless_input_core::control::InputKey;
use headless_input_core::run::{EndReason, RunId, RunState};
use headless_input_core::structs::Capabilities;
use headless_input_mcp::port::{HeadlessPort, PortError};

use crate::support::fake_command_client::FakeCommandClient;
use crate::support::fake_join_client::FakeJoinClient;
use crate::support::fake_manager::{Call, FakeManager};
use crate::support::{RunHarness, RuntimeDirs};

fn runtime() -> HeadlessRuntime {
    HeadlessRuntime::new(&RuntimeDirs::dirs())
}

fn forward(down: bool) -> Call {
    Call::Key(KeyCodes::key(InputKey::Forward), down)
}

const WAIT: &str = "name = \"wait\"\n\nstep \"wait\" {\n  for = \"5s\"\n}\n";

// a runtime attached to the fake manager table, driven through its own link
struct Linked {
    runtime: HeadlessRuntime,
    join: FakeJoinClient,
    command: FakeCommandClient,
}

impl Linked {
    fn new() -> Self {
        let runtime = runtime();
        runtime.enable_link();
        runtime.tick_with(|| Ok(FakeTable::table()));
        assert_eq!(runtime.link_state(), LinkState::Attached);
        Self { runtime, join: FakeJoinClient::in_world(), command: FakeCommandClient::new() }
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

    // the manager events, then the frame, as one ClientInstance::update runs them
    fn update(&self, frame: &FrameView) {
        self.runtime.tick_with(|| Err(LinkError::ManagerMissing));
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

    fn sent_keys() -> Vec<(u32, bool)> {
        FakeTable::with(|state| state.sent_keys())
    }

    fn released() -> Vec<Capability> {
        FakeTable::with(|state| state.released.clone())
    }
}

fn w() -> u32 {
    KeyCodes::key(InputKey::Forward)
}

#[test]
fn the_saved_settings_are_read_on_attach_and_again_on_change() {
    FakeTable::set_config("mcp {\n  port = 29001\n}\n");
    let runtime = runtime();
    runtime.enable_link();
    runtime.tick_with(|| Ok(FakeTable::table()));
    assert_eq!(runtime.link_state(), LinkState::Attached);
    assert_eq!(runtime.config().mcp_port, 29001);

    FakeTable::set_config("mcp {\n  port = 29002\n}\n");
    FakeTable::push_event(ModEvent::SettingsChanged { paths: 1 });
    runtime.tick_with(|| Err(LinkError::ManagerMissing));

    assert_eq!(runtime.config().mcp_port, 29002);
}

#[test]
fn a_missing_manager_leaves_the_defaults() {
    let runtime = runtime();
    runtime.enable_link();

    runtime.tick_with(|| Err(LinkError::ManagerMissing));

    assert_eq!(runtime.link_state(), LinkState::Off);
    assert_eq!(runtime.config().mcp_port, 28250);
}

#[test]
fn a_tick_before_enable_does_nothing() {
    let runtime = runtime();
    let called = AtomicBool::new(false);

    runtime.tick_with(|| {
        called.store(true, Ordering::SeqCst);
        Err(LinkError::ManagerMissing)
    });

    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(runtime.link_state(), LinkState::Disabled);
}

#[test]
fn detach_drops_the_link_and_stops_ticks_reaching_the_manager() {
    let runtime = runtime();
    runtime.enable_link();
    runtime.tick_with(|| Ok(FakeTable::table()));
    runtime.detach();

    runtime.tick_with(|| Ok(FakeTable::table()));

    assert_eq!(runtime.link_state(), LinkState::Disabled);
}

#[test]
fn capture_keeps_the_first_non_null_client() {
    let runtime = runtime();
    runtime.capture(0);
    assert_eq!(runtime.client(), 0);
    runtime.capture(0x10);
    runtime.capture(0x20);
    assert_eq!(runtime.client(), 0x10);
}

#[test]
fn hooks_are_bounded_and_taken_newest_first() {
    let runtime = runtime();
    for id in 0..HeadlessRuntime::HOOK_CAPACITY as u64 {
        assert!(runtime.keep_hook(HookHandle(id)));
    }
    assert!(!runtime.keep_hook(HookHandle(99)));

    let ids: Vec<u64> = runtime.take_hooks().iter().map(|hook| hook.0).collect();
    assert_eq!(ids, vec![7, 6, 5, 4, 3, 2, 1, 0]);
    assert!(runtime.take_hooks().is_empty());
}

#[test]
fn capabilities_are_off_until_the_entries_are_set() {
    assert_eq!(runtime().capabilities(), Capabilities::new(false, false, false, false, false, false));
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
    assert_eq!(HeadlessRuntime::instance_id(&RuntimeDirs::dirs()).as_str(), RuntimeDirs::INSTANCE);

    let loose = ModDirs {
        mod_dir: PathBuf::from("D:\\loose\\headless-input"),
        config_dir: PathBuf::from("D:\\loose\\config"),
        data_dir: PathBuf::from("D:\\loose\\data"),
    };
    let fallback = HeadlessRuntime::instance_id(&loose);
    assert!(fallback.as_str().starts_with("instance-"), "{fallback}");
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

    harness.runtime.enable_link();
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
    assert_eq!(Linked::released(), vec![Capability::Input]);
    assert!(Linked::sent_keys().is_empty());

    FakeTable::with(|state| state.panel_open = false);
    linked.tick();
    linked.update(&RunHarness::world());
    assert_eq!(linked.state(), RunState::Idle);
    assert_eq!(FakeTable::with(|state| state.acquired.clone()), vec![Capability::Input]);
}

#[test]
fn another_mod_suspending_the_input_aborts_the_run_and_releases_keys_then_lease() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    FakeTable::push_event(ModEvent::Suspended { capability: Capability::Input, by: ModIndex(3) });

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Idle);
    let reason = linked.reason();
    assert!(
        matches!(reason, Some(EndReason::Aborted { ref why }) if why == "another mod took the input"),
        "{reason:?}"
    );
    assert_eq!(Linked::sent_keys(), vec![(w(), true), (w(), false)]);
    assert_eq!(Linked::released(), vec![Capability::Input]);
}

#[test]
fn another_mod_revoking_the_input_aborts_the_run() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    FakeTable::push_event(ModEvent::Revoked { capability: Capability::Input, by: ModIndex(3) });

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Idle);
    let reason = linked.reason();
    assert!(
        matches!(reason, Some(EndReason::Aborted { ref why }) if why == "another mod took the input"),
        "{reason:?}"
    );
    assert_eq!(Linked::sent_keys(), vec![(w(), true), (w(), false)]);
}

#[test]
fn the_manager_disabling_the_mod_aborts_the_run_and_releases_it() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    FakeTable::push_event(ModEvent::Disabled);

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Idle);
    let reason = linked.reason();
    assert!(matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the mod was disabled"), "{reason:?}");
    assert_eq!(Linked::sent_keys(), vec![(w(), true), (w(), false)]);
    assert_eq!(Linked::released(), vec![Capability::Input]);
}

#[test]
fn a_suspension_of_another_capability_leaves_the_run() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    FakeTable::push_event(ModEvent::Suspended { capability: Capability::Camera, by: ModIndex(3) });

    linked.update(&RunHarness::world());

    assert_eq!(linked.state(), RunState::Running);
    assert!(Linked::released().is_empty());
}

#[test]
fn fake_keys_released_drops_the_record_and_a_world_exit_on_that_frame_aborts() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();
    FakeTable::push_event(ModEvent::FakeKeysReleased);
    linked.update(&RunHarness::world());
    assert_eq!(linked.state(), RunState::Running, "a release of the fake keys is not the panel");
    linked.tick();
    assert_eq!(Linked::sent_keys(), vec![(w(), true), (w(), true)], "the forgotten key goes down again");

    FakeTable::push_event(ModEvent::FakeKeysReleased);
    linked.update(&RunHarness::title());

    let reason = linked.reason();
    assert!(matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the world was left"), "{reason:?}");
    assert_eq!(Linked::sent_keys(), vec![(w(), true), (w(), true)], "no up for a key the manager let go");
    assert_eq!(Linked::released(), vec![Capability::Input]);
}

#[test]
fn a_panic_in_a_hook_body_ends_the_run_and_releases_keys_then_lease() {
    let linked = Linked::new();
    linked.start(RunHarness::WALK);
    linked.tick();

    linked.runtime.guarded(|| panic!("forced panic in a hook body"));

    assert_eq!(Linked::sent_keys(), vec![(w(), true), (w(), false)]);
    assert_eq!(Linked::released(), vec![Capability::Input]);
    assert_eq!(linked.runtime.link_state(), LinkState::Disabled);
    let reason = linked.reason();
    let internal = matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the mod met an internal error");
    assert!(internal, "{reason:?}");
    linked.tick();
    linked.update(&RunHarness::world());
    assert_eq!(Linked::sent_keys().len(), 2, "nothing runs after the panic");
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
    linked.runtime.tick_with(|| Err(LinkError::ManagerMissing));
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
    linked.runtime.tick_with(|| Err(LinkError::ManagerMissing));
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
    linked.runtime.tick_with(|| Err(LinkError::ManagerMissing));
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
    linked.runtime.tick_with(|| Err(LinkError::ManagerMissing));
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
        linked.runtime.tick_with(|| Err(LinkError::ManagerMissing));
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
