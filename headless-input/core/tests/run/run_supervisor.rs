use std::net::Ipv4Addr;
use std::time::Duration;

use headless_input_core::control::{ControlFile, DimensionName, InputKey, PointerAction, Step, Ticks};
use headless_input_core::errors::Refusal;
use headless_input_core::player::{InputFrame, KeySet, PlayerView, PointerOp, ScreenKind};
use headless_input_core::run::{Command, EndReason, FrameOutcome, LiveAction, RunDefaults, RunState, RunSupervisor};

use crate::support::FakeClock;

const SECOND: Duration = Duration::from_secs(1);

fn supervisor() -> (RunSupervisor<FakeClock>, FakeClock) {
    let clock = FakeClock::new();
    let defaults = RunDefaults { goal_timeout: Ticks(600), watchdog: Ticks(100), live_idle: Ticks(600) };
    (RunSupervisor::new(defaults, clock.clone()), clock)
}

fn file(steps: Vec<Step>) -> Command {
    let total_ticks = None;
    Command::RunFile { file: ControlFile { name: "test".to_string(), steps, total_ticks } }
}

fn fixed_file(steps: Vec<Step>, total: u32) -> Command {
    Command::RunFile { file: ControlFile { name: "test".to_string(), steps, total_ticks: Some(total) } }
}

fn walk(ticks: u32) -> Step {
    Step::Move { hold: vec![InputKey::Forward], ticks: Ticks(ticks) }
}

fn view(screen: ScreenKind) -> PlayerView {
    PlayerView { position: [1.0, 64.0, 2.0], yaw: 0.0, pitch: 0.0, dimension: Some(0), screen }
}

fn world() -> PlayerView {
    view(ScreenKind::InWorld)
}

fn keys(list: &[InputKey]) -> KeySet {
    let mut set = KeySet::new();
    for key in list {
        set.insert(*key);
    }
    set
}

fn end(sup: &mut RunSupervisor<FakeClock>, view: &PlayerView, panel_open: bool, in_world: bool) -> Option<EndReason> {
    let outcome = sup.on_frame(view, panel_open, in_world);
    if outcome.end.is_some() {
        assert_eq!(outcome.frame, Some(InputFrame::default()), "an end comes with the empty frame");
    }
    outcome.end
}

fn title() -> PlayerView {
    view(ScreenKind::Title)
}

fn loading() -> PlayerView {
    view(ScreenKind::Loading)
}

fn join(timeout: u32) -> Step {
    Step::JoinServer {
        host: "play.example.net".to_string(),
        address: Ipv4Addr::new(10, 0, 0, 2),
        port: 19132,
        timeout: Ticks(timeout),
    }
}

fn live_join(timeout: u32) -> LiveAction {
    LiveAction::JoinServer {
        host: "play.example.net".to_string(),
        address: Ipv4Addr::new(10, 0, 0, 2),
        port: 19132,
        timeout: Some(Ticks(timeout)),
    }
}

#[test]
fn a_live_join_is_not_ended_by_the_idle_timer() {
    let clock = FakeClock::new();
    let defaults = RunDefaults { goal_timeout: Ticks(600), watchdog: Ticks(100), live_idle: Ticks(600) };
    let mut sup = RunSupervisor::new(defaults, clock.clone());
    sup.apply(Command::Live(live_join(1200))).expect("the session starts");
    assert!(sup.on_frame(&title(), false, false).frame.expect("a frame-built input").join.is_some());
    for _ in 0..40 {
        clock.advance(SECOND);
        assert_eq!(end(&mut sup, &loading(), false, false), None);
    }
    assert_eq!(sup.status(&loading()).state, RunState::Live);
}

#[test]
fn a_join_that_returns_to_the_menus_after_loading_fails() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![join(600), walk(10)])).expect("the run starts");
    assert!(sup.on_frame(&title(), false, false).frame.expect("a frame-built input").join.is_some());
    assert!(sup.on_frame(&loading(), false, false).frame.expect("a frame-built input").join.is_some());
    match end(&mut sup, &title(), false, false) {
        Some(EndReason::Failed { step, why, .. }) => {
            assert_eq!(step, Some(0));
            assert_eq!(why, "the join failed: the game returned to the menus");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn a_live_join_that_returns_to_the_menus_after_loading_fails() {
    let (mut sup, _clock) = supervisor();
    sup.apply(Command::Live(live_join(1200))).expect("the session starts");
    sup.on_frame(&title(), false, false);
    sup.on_frame(&loading(), false, false);
    match end(&mut sup, &title(), false, false) {
        Some(EndReason::Failed { why, .. }) => assert_eq!(why, "the join failed: the game returned to the menus"),
        other => panic!("expected a failure, got {other:?}"),
    }
    assert_eq!(sup.status(&title()).state, RunState::Idle);
}

#[test]
fn a_loading_screen_before_the_join_is_asked_does_not_fail_it() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![join(600), walk(10)])).expect("the run starts");
    assert_eq!(end(&mut sup, &loading(), false, false), None);
    assert!(sup.on_frame(&title(), false, false).frame.expect("a frame-built input").join.is_some());
    assert_eq!(sup.on_frame(&world(), false, true), FrameOutcome { frame: None, end: None });
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(sup.status(&world()).cursor.map(|cursor| cursor.step), Some(1));
}

#[test]
fn a_join_that_reaches_the_world_ends() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![join(600), walk(10)])).expect("the run starts");
    sup.on_frame(&title(), false, false);
    sup.on_frame(&loading(), false, false);
    assert_eq!(sup.on_frame(&world(), false, true), FrameOutcome { frame: None, end: None });
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(sup.status(&world()).cursor.map(|cursor| cursor.step), Some(1));
}

#[test]
fn a_run_ends_done_after_the_last_step() {
    let (mut sup, _clock) = supervisor();
    let id = sup.apply(fixed_file(vec![walk(2)], 2)).expect("the run starts");
    assert_eq!(sup.status(&world()).state, RunState::Running);
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(end(&mut sup, &world(), false, true), None);
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
    assert_eq!(end(&mut sup, &world(), false, true), Some(EndReason::Done));
    let result = sup.take_end().expect("a result");
    assert_eq!(result.run, id);
    assert_eq!(result.reason, EndReason::Done);
    assert!(sup.take_end().is_none());
    assert_eq!(end(&mut sup, &world(), false, true), None);
    let status = sup.status(&world());
    assert_eq!(status.state, RunState::Idle);
    assert_eq!(status.last.as_ref().map(|last| last.reason.clone()), Some(EndReason::Done));
}

#[test]
fn stop_ends_the_run_stopped_and_empties_the_frame() {
    let (mut sup, _clock) = supervisor();
    let id = sup.apply(file(vec![walk(100)])).expect("the run starts");
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(sup.apply(Command::Stop), Ok(id));
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
    assert_eq!(end(&mut sup, &world(), false, true), Some(EndReason::Stopped));
    assert_eq!(sup.take_end().map(|result| result.reason), Some(EndReason::Stopped));
    assert_eq!(sup.apply(Command::Stop), Err(Refusal::NothingActive));
}

#[test]
fn no_tick_for_the_watchdog_time_aborts() {
    let (mut sup, clock) = supervisor();
    sup.apply(file(vec![walk(1000)])).expect("the run starts");
    sup.on_tick(&world());
    clock.advance(SECOND * 4);
    assert_eq!(end(&mut sup, &world(), false, true), None);
    clock.advance(SECOND);
    assert!(matches!(end(&mut sup, &world(), false, true), Some(EndReason::Aborted { .. })));
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
}

#[test]
fn the_watchdog_pauses_during_join_server() {
    let (mut sup, clock) = supervisor();
    sup.apply(file(vec![join(600), walk(1000)])).expect("the run starts");
    let at_title = sup.on_frame(&title(), false, false);
    assert_eq!(at_title.end, None);
    assert!(at_title.frame.expect("a frame-built input").join.is_some());
    clock.advance(SECOND * 8);
    let loading_frame = sup.on_frame(&loading(), false, false);
    assert_eq!(loading_frame.end, None);
    assert!(loading_frame.frame.expect("a frame-built input").join.is_some());
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(end(&mut sup, &world(), false, true), None);
    clock.advance(SECOND * 5);
    assert!(matches!(end(&mut sup, &world(), false, true), Some(EndReason::Aborted { .. })));
}

#[test]
fn wall_clock_past_twice_the_length_aborts() {
    let (mut sup, clock) = supervisor();
    sup.apply(fixed_file(vec![walk(100)], 100)).expect("the run starts");
    for _ in 0..9 {
        sup.on_tick(&world());
        clock.advance(SECOND);
        assert_eq!(end(&mut sup, &world(), false, true), None);
    }
    sup.on_tick(&world());
    clock.advance(SECOND);
    assert!(matches!(end(&mut sup, &world(), false, true), Some(EndReason::Aborted { .. })));
}

#[test]
fn a_screen_over_the_world_fails_and_names_the_screen() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![Step::Wait { ticks: Ticks(5) }, walk(100)])).expect("the run starts");
    for _ in 0..6 {
        sup.on_tick(&world());
    }
    let pause = view(ScreenKind::Other("pause_screen".to_string()));
    match end(&mut sup, &pause, false, true) {
        Some(EndReason::Failed { step, view, why }) => {
            assert_eq!(step, Some(1));
            assert_eq!(view.screen, ScreenKind::Other("pause_screen".to_string()));
            assert!(why.contains("pause_screen"), "{why}");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn loading_screens_during_join_do_not_fail() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![join(600), walk(10)])).expect("the run starts");
    assert!(sup.on_tick(&world()).leave);
    assert_eq!(end(&mut sup, &view(ScreenKind::Other("disconnect_screen".to_string())), false, true), None);
    let leaving = sup.on_frame(&loading(), false, false);
    assert_eq!(leaving.end, None);
    assert!(leaving.frame.expect("a frame-built input").leave);
    let joining = sup.on_frame(&title(), false, false);
    assert_eq!(joining.end, None);
    assert!(joining.frame.expect("a frame-built input").join.is_some());
    let still_joining = sup.on_frame(&loading(), false, false);
    assert!(still_joining.frame.expect("a frame-built input").join.is_some());
    let in_world = sup.on_frame(&view(ScreenKind::Other("server_form".to_string())), false, true);
    assert_eq!(in_world, FrameOutcome { frame: None, end: None });
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(end(&mut sup, &world(), false, true), None);
    assert_eq!(sup.status(&world()).state, RunState::Running);
}

#[test]
fn an_unknown_dimension_while_loading_does_not_end_a_walk_to_the_overworld() {
    let (mut sup, _clock) = supervisor();
    let step = Step::WalkUntilDimension { dimension: Some(DimensionName::Overworld), timeout: Ticks(600) };
    sup.apply(file(vec![step])).expect("the run starts");
    let mut nether = world();
    nether.dimension = Some(1);
    assert!(sup.on_tick(&nether).keys.contains(InputKey::Forward));
    assert_eq!(end(&mut sup, &nether, false, true), None);
    let mut unknown = loading();
    unknown.dimension = None;
    for _ in 0..5 {
        assert!(sup.on_tick(&unknown).keys.contains(InputKey::Forward));
        assert_eq!(end(&mut sup, &unknown, false, true), None);
    }
    assert!(sup.on_tick(&world()).keys.is_empty());
    assert_eq!(end(&mut sup, &world(), false, true), Some(EndReason::Done));
}

#[test]
fn leaving_the_world_during_a_walk_aborts() {
    let (mut sup, _clock) = supervisor();
    let step = Step::WalkUntilDimension { dimension: None, timeout: Ticks(600) };
    sup.apply(file(vec![step])).expect("the run starts");
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(end(&mut sup, &world(), false, true), None);
    let reason = end(&mut sup, &view(ScreenKind::Title), false, false);
    assert!(matches!(reason, Some(EndReason::Aborted { .. })), "{reason:?}");
    assert_eq!(sup.on_tick(&view(ScreenKind::Title)), InputFrame::default());
}

#[test]
fn leaving_the_world_while_the_panel_reads_open_still_aborts() {
    let (mut sup, _clock) = supervisor();
    let step = Step::WalkUntilDimension { dimension: None, timeout: Ticks(600) };
    sup.apply(file(vec![step])).expect("the run starts");
    sup.on_tick(&world());
    assert_eq!(end(&mut sup, &world(), false, true), None);

    let reason = end(&mut sup, &title(), true, false);

    assert!(matches!(reason, Some(EndReason::Aborted { ref why }) if why == "the world was left"), "{reason:?}");
}

#[test]
fn leaving_during_leave_world_is_done() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![Step::LeaveWorld { timeout: Ticks(600) }])).expect("the run starts");
    assert!(sup.on_tick(&world()).leave);
    assert_eq!(end(&mut sup, &world(), false, true), None);
    let leaving = sup.on_frame(&loading(), false, false);
    assert_eq!(leaving.end, None);
    assert!(leaving.frame.expect("a frame-built input").leave);
    assert_eq!(end(&mut sup, &title(), false, false), Some(EndReason::Done));
}

#[test]
fn a_first_step_join_from_the_title_requests_the_join_and_ends_in_world() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![join(600), walk(10)])).expect("the run starts");
    assert!(sup.on_frame(&title(), false, false).frame.expect("a frame-built input").join.is_some());
    assert!(sup.on_frame(&loading(), false, false).frame.expect("a frame-built input").join.is_some());
    assert_eq!(sup.on_frame(&world(), false, true), FrameOutcome { frame: None, end: None });
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(sup.status(&world()).cursor.map(|cursor| cursor.step), Some(1));
}

#[test]
fn a_first_step_join_waits_out_the_watchdog_and_ends_on_its_timeout() {
    let (mut sup, clock) = supervisor();
    sup.apply(file(vec![join(200)])).expect("the run starts");
    for _ in 0..9 {
        clock.advance(SECOND);
        assert_eq!(end(&mut sup, &title(), false, false), None);
    }
    clock.advance(SECOND);
    match end(&mut sup, &loading(), false, false) {
        Some(EndReason::Failed { step, why, .. }) => {
            assert_eq!(step, Some(0));
            assert!(why.contains("join_server"), "{why}");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn a_frame_after_a_tick_does_not_advance_the_step() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![Step::LeaveWorld { timeout: Ticks(600) }])).expect("the run starts");
    assert!(sup.on_tick(&loading()).leave);
    assert_eq!(sup.on_frame(&title(), false, false), FrameOutcome { frame: None, end: None });
    assert_eq!(end(&mut sup, &title(), false, false), Some(EndReason::Done));
}

#[test]
fn a_live_join_is_driven_by_frames_at_the_title() {
    let (mut sup, clock) = supervisor();
    let action = LiveAction::JoinServer {
        host: "play.example.net".to_string(),
        address: Ipv4Addr::new(10, 0, 0, 2),
        port: 19132,
        timeout: None,
    };
    sup.apply(Command::Live(action)).expect("the session starts");
    assert!(sup.on_frame(&title(), false, false).frame.expect("a frame-built input").join.is_some());
    clock.advance(SECOND * 6);
    assert!(sup.on_frame(&loading(), false, false).frame.expect("a frame-built input").join.is_some());
    sup.on_tick(&world());
    assert_eq!(sup.on_frame(&world(), false, true), FrameOutcome { frame: None, end: None });
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
    assert_eq!(sup.status(&world()).state, RunState::Live);
}

#[test]
fn the_panel_opening_stops_the_run() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![walk(100)])).expect("the run starts");
    sup.on_tick(&world());
    assert_eq!(end(&mut sup, &world(), true, true), Some(EndReason::Stopped));
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
}

#[test]
fn join_server_times_out_on_the_wall_clock() {
    let (mut sup, clock) = supervisor();
    sup.apply(file(vec![join(200)])).expect("the run starts");
    assert!(sup.on_tick(&world()).leave);
    clock.advance(SECOND * 9);
    assert_eq!(end(&mut sup, &view(ScreenKind::Loading), false, false), None);
    clock.advance(SECOND);
    match end(&mut sup, &view(ScreenKind::Loading), false, false) {
        Some(EndReason::Failed { step, why, .. }) => {
            assert_eq!(step, Some(0));
            assert!(why.contains("join_server"), "{why}");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn abort_ends_the_run_aborted() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![walk(100)])).expect("the run starts");
    sup.on_tick(&world());
    sup.abort("the mod was disabled");
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
    assert_eq!(end(&mut sup, &world(), false, true), Some(EndReason::Aborted { why: "the mod was disabled".into() }));
}

#[test]
fn live_actions_start_a_session_and_idle_ends_it() {
    let (mut sup, clock) = supervisor();
    let hold = LiveAction::Hold { keys: keys(&[InputKey::Forward]), ticks: None };
    sup.apply(Command::Live(hold)).expect("the session starts");
    assert_eq!(sup.status(&world()).state, RunState::Live);
    sup.apply(Command::Live(LiveAction::Hotbar { slot: 3 })).expect("the session takes it");
    let frame = sup.on_tick(&world());
    assert!(frame.keys.contains(InputKey::Forward));
    assert_eq!(frame.hotbar, Some(3));
    assert_eq!(sup.on_tick(&world()).hotbar, None);
    for _ in 0..29 {
        clock.advance(SECOND);
        assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
        assert_eq!(end(&mut sup, &world(), false, true), None);
    }
    clock.advance(SECOND);
    sup.on_tick(&world());
    assert!(matches!(end(&mut sup, &world(), false, true), Some(EndReason::Aborted { .. })));
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
    assert_eq!(sup.status(&world()).state, RunState::Idle);
}

#[test]
fn live_holds_with_ticks_end_and_a_click_releases_next_tick() {
    let (mut sup, _clock) = supervisor();
    sup.apply(Command::Live(LiveAction::Hold { keys: keys(&[InputKey::Jump]), ticks: Some(Ticks(2)) }))
        .expect("the session starts");
    sup.apply(Command::Live(LiveAction::Pointer { action: PointerAction::Attack, hold_ticks: 0 }))
        .expect("the session takes it");
    let first = sup.on_tick(&world());
    assert!(first.keys.contains(InputKey::Jump));
    assert_eq!(first.pointer.as_slice(), &[PointerOp::Press(PointerAction::Attack)]);
    let second = sup.on_tick(&world());
    assert!(second.keys.contains(InputKey::Jump));
    assert_eq!(second.pointer.as_slice(), &[PointerOp::Release(PointerAction::Attack)]);
    let third = sup.on_tick(&world());
    assert!(third.keys.is_empty());
    assert!(third.pointer.is_empty());
}

#[test]
fn release_ends_the_live_session() {
    let (mut sup, _clock) = supervisor();
    let hold = LiveAction::Hold { keys: keys(&[InputKey::Forward]), ticks: None };
    let id = sup.apply(Command::Live(hold)).expect("the session starts");
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(sup.apply(Command::Live(LiveAction::Release)), Ok(id));
    assert_eq!(sup.on_tick(&world()), InputFrame::default());
    assert_eq!(end(&mut sup, &world(), false, true), Some(EndReason::Done));
    assert_eq!(sup.take_end().map(|result| result.run), Some(id));
}

#[test]
fn a_file_run_is_refused_during_a_live_session() {
    let (mut sup, _clock) = supervisor();
    sup.apply(Command::Live(LiveAction::Hotbar { slot: 1 })).expect("the session starts");
    assert_eq!(sup.apply(file(vec![walk(10)])), Err(Refusal::LiveActive));
    assert_eq!(sup.status(&world()).state, RunState::Live);
}

#[test]
fn a_live_action_is_refused_during_a_file_run() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![walk(10)])).expect("the run starts");
    assert_eq!(sup.apply(Command::Live(LiveAction::Hotbar { slot: 1 })), Err(Refusal::RunActive));
    assert_eq!(sup.apply(Command::Live(LiveAction::Release)), Err(Refusal::RunActive));
}

#[test]
fn a_second_file_run_is_refused() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![walk(10)])).expect("the run starts");
    assert_eq!(sup.apply(file(vec![walk(10)])), Err(Refusal::RunActive));
}

#[test]
fn a_new_run_waits_until_the_last_end_is_reported() {
    let (mut sup, _clock) = supervisor();
    let first = sup.apply(file(vec![walk(10)])).expect("the run starts");
    sup.apply(Command::Stop).expect("the run stops");
    assert_eq!(sup.apply(file(vec![walk(10)])), Err(Refusal::Ending));
    assert_eq!(end(&mut sup, &world(), false, true), Some(EndReason::Stopped));
    let second = sup.apply(file(vec![walk(10)])).expect("the next run starts");
    assert_ne!(first, second);
}

fn portal_walk(sup: &mut RunSupervisor<FakeClock>) {
    let step = Step::WalkUntilDimension { dimension: None, timeout: Ticks(600) };
    sup.apply(file(vec![step])).expect("the run starts");
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(end(sup, &world(), false, true), None);
}

#[test]
fn a_portal_load_with_no_tick_and_no_player_does_not_end_a_walk() {
    let (mut sup, clock) = supervisor();
    portal_walk(&mut sup);
    let mut gone = loading();
    gone.dimension = None;
    for _ in 0..20 {
        clock.advance(SECOND / 2);
        assert_eq!(end(&mut sup, &gone, false, false), None);
    }
    clock.advance(SECOND / 2);
    let mut nether = world();
    nether.dimension = Some(1);
    assert_eq!(end(&mut sup, &nether, false, true), None);
    assert!(sup.on_tick(&nether).keys.is_empty());
    assert_eq!(end(&mut sup, &nether, false, true), Some(EndReason::Done));
}

#[test]
fn a_title_frame_during_a_portal_walk_aborts_as_world_left() {
    let (mut sup, _clock) = supervisor();
    portal_walk(&mut sup);
    assert_eq!(end(&mut sup, &loading(), false, false), None);
    let reason = end(&mut sup, &title(), false, false);
    assert_eq!(reason, Some(EndReason::Aborted { why: "the world was left".into() }));
}

#[test]
fn the_wall_clock_cap_ends_a_portal_walk_stuck_on_loading() {
    let (mut sup, clock) = supervisor();
    portal_walk(&mut sup);
    let mut reason = None;
    for _ in 0..200 {
        clock.advance(SECOND);
        reason = end(&mut sup, &loading(), false, false);
        if reason.is_some() {
            break;
        }
    }
    assert_eq!(reason, Some(EndReason::Aborted { why: "the run passed twice its length".into() }));
}

#[test]
fn loading_during_a_move_step_still_obeys_the_watchdog() {
    let (mut sup, clock) = supervisor();
    sup.apply(file(vec![walk(600)])).expect("the run starts");
    assert!(sup.on_tick(&world()).keys.contains(InputKey::Forward));
    assert_eq!(end(&mut sup, &world(), false, true), None);
    clock.advance(SECOND * 6);
    let reason = end(&mut sup, &loading(), false, true);
    assert_eq!(reason, Some(EndReason::Aborted { why: "no game tick for the watchdog time".into() }));
}

#[test]
fn a_failure_code_from_the_game_fails_the_join_step_and_names_the_code() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![join(600), walk(10)])).expect("the run starts");
    assert!(sup.on_frame(&title(), false, false).frame.expect("a frame-built input").join.is_some());

    assert!(sup.join_refused(&title(), 8));

    assert_eq!(sup.status(&title()).state, RunState::Idle);
    match end(&mut sup, &title(), false, false) {
        Some(EndReason::Failed { step, why, .. }) => {
            assert_eq!(step, Some(0));
            assert!(why.contains("could not resolve") && why.contains('8'), "{why}");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn each_failure_code_names_itself_in_the_reason() {
    for code in [1, 5, 6, 7, 9, 0x15] {
        let (mut sup, _clock) = supervisor();
        sup.apply(Command::Live(live_join(600))).expect("the session starts");
        assert!(sup.join_refused(&title(), code), "code {code}");
        match end(&mut sup, &title(), false, false) {
            Some(EndReason::Failed { step: None, why, .. }) => assert!(why.contains(&code.to_string()), "{why}"),
            other => panic!("expected a failure for {code}, got {other:?}"),
        }
    }
}

#[test]
fn a_code_that_is_no_failure_leaves_the_join_running() {
    let (mut sup, _clock) = supervisor();
    sup.apply(file(vec![join(600)])).expect("the run starts");
    sup.on_frame(&title(), false, false);

    assert!(!sup.join_refused(&title(), 0));

    assert_eq!(sup.status(&title()).state, RunState::Running);
}

#[test]
fn a_failure_code_with_no_join_under_way_ends_nothing() {
    let (mut sup, _clock) = supervisor();
    assert!(!sup.join_refused(&title(), 8));
    sup.apply(file(vec![walk(10)])).expect("the run starts");

    assert!(!sup.join_refused(&world(), 8));

    assert_eq!(sup.status(&world()).state, RunState::Running);
}
