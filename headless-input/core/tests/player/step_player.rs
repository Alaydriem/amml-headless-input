use headless_input_core::control::{ControlFile, ControlLoader, DimensionName, InputKey, PointerAction, Step, Ticks};
use headless_input_core::player::{InputFrame, PlayerView, PointerOp, ScreenKind, StepOutcome, StepPlayer};
use headless_input_core::run::RunDefaults;
use headless_input_core::structs::Capabilities;
use std::net::Ipv4Addr;
use std::sync::Arc;

use headless_input_core::structs::JoinTarget;

fn player(steps: Vec<Step>) -> StepPlayer {
    StepPlayer::new(ControlFile { name: "test".to_string(), steps, total_ticks: None })
}

fn view_at(x: f32, z: f32, yaw: f32) -> PlayerView {
    PlayerView { position: [x, 64.0, z], yaw, pitch: 0.0, dimension: Some(0), screen: ScreenKind::InWorld }
}

fn still() -> PlayerView {
    view_at(0.0, 0.0, 0.0)
}

fn mv(keys: &[InputKey], ticks: u32) -> Step {
    Step::Move { hold: keys.to_vec(), ticks: Ticks(ticks) }
}

fn angle(a: f32, b: f32) -> f32 {
    (a - b + 180.0).rem_euclid(360.0) - 180.0
}

fn yaw_of(frame: &InputFrame) -> f32 {
    frame.rotation.expect("a rotation").0
}

#[test]
fn move_holds_its_keys_for_exactly_its_ticks() {
    let mut player = player(vec![mv(&[InputKey::Forward], 100), Step::Wait { ticks: Ticks(5) }]);
    for tick in 0..100 {
        let frame = player.tick(&still());
        assert!(frame.keys.contains(InputKey::Forward), "tick {tick}");
    }
    let frame = player.tick(&still());
    assert!(!frame.keys.contains(InputKey::Forward));
    assert!(frame.keys.is_empty());
    assert_eq!(player.current().ticks_in_step, 1);
}

#[test]
fn keys_are_released_between_steps_unless_the_next_step_holds_them() {
    let mut player = player(vec![
        mv(&[InputKey::Forward, InputKey::Left], 2),
        mv(&[InputKey::Forward], 2),
        mv(&[InputKey::Back], 2),
    ]);
    let frames: Vec<InputFrame> = (0..6).map(|_| player.tick(&still())).collect();
    assert!(frames[1].keys.contains(InputKey::Left));
    assert!(frames[2].keys.contains(InputKey::Forward));
    assert!(!frames[2].keys.contains(InputKey::Left));
    assert!(frames[4].keys.contains(InputKey::Back));
    assert!(!frames[4].keys.contains(InputKey::Forward));
}

#[test]
fn jump_presses_for_one_tick_by_default() {
    let file = ControlLoader::load(
        "name = \"j\"\n\nstep \"jump\" {}\n\nstep \"wait\" {\n  for = \"5t\"\n}\n",
        &RunDefaults::default(),
        &Capabilities::all(),
        |_| Some(Ipv4Addr::LOCALHOST),
    )
    .expect("the file loads");
    let mut player = StepPlayer::new(file);
    assert!(player.tick(&still()).keys.contains(InputKey::Jump));
    assert!(!player.tick(&still()).keys.contains(InputKey::Jump));
}

#[test]
fn look_at_reaches_the_target_rotation_on_its_last_tick() {
    let mut player = player(vec![Step::LookAt { yaw: 90.0, pitch: 30.0, ticks: Ticks(10) }]);
    let frames: Vec<InputFrame> = (0..10).map(|_| player.tick(&still())).collect();
    let first = frames[0].rotation.expect("a rotation");
    assert!(first.0 > 0.0 && first.0 < 90.0);
    assert!(first.1 > 0.0 && first.1 < 30.0);
    assert_eq!(frames[9].rotation, Some((90.0, 30.0)));
    assert_eq!(player.tick(&still()).rotation, None);
}

#[test]
fn turn_adds_to_the_current_rotation() {
    let mut player = player(vec![Step::Turn { yaw: 45.0, pitch: 10.0, ticks: Ticks(5) }]);
    let start = PlayerView { pitch: 5.0, ..view_at(0.0, 0.0, 10.0) };
    let frames: Vec<InputFrame> = (0..5).map(|_| player.tick(&start)).collect();
    let last = frames[4].rotation.expect("a rotation");
    assert!((last.0 - 55.0).abs() < 1e-3);
    assert!((last.1 - 15.0).abs() < 1e-3);
}

#[test]
fn yaw_takes_the_short_way_round() {
    let mut player = player(vec![Step::LookAt { yaw: 10.0, pitch: 0.0, ticks: Ticks(4) }]);
    let start = view_at(0.0, 0.0, 350.0);
    let frames: Vec<InputFrame> = (0..4).map(|_| player.tick(&start)).collect();
    assert!((angle(yaw_of(&frames[0]), 350.0) - 5.0).abs() < 1e-3);
    assert!((angle(yaw_of(&frames[1]), 350.0) - 10.0).abs() < 1e-3);
    assert!(angle(yaw_of(&frames[3]), 10.0).abs() < 1e-3);
}

#[test]
fn walk_to_turns_first_then_walks() {
    let mut player =
        player(vec![Step::WalkTo { x: 0.0, z: -10.0, y: None, within: 0.5, sprint: true, timeout: Ticks(600) }]);
    let mut view = still();
    for tick in 0..5 {
        let frame = player.tick(&view);
        assert!(frame.keys.is_empty(), "tick {tick} holds a key while turning");
        view.yaw = yaw_of(&frame);
    }
    assert!(angle(view.yaw, 180.0).abs() < 1e-3);
    for z in 0..9 {
        view.position[2] = -(z as f32);
        let frame = player.tick(&view);
        assert!(frame.keys.contains(InputKey::Forward));
        assert!(frame.keys.contains(InputKey::Sprint));
        assert!(!frame.keys.contains(InputKey::Back));
        assert!(angle(yaw_of(&frame), 180.0).abs() < 1e-3);
    }
    view.position[2] = -9.7;
    let frame = player.tick(&view);
    assert!(frame.keys.is_empty());
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn walk_to_times_out_with_the_view() {
    let mut player =
        player(vec![Step::WalkTo { x: 0.0, z: 50.0, y: None, within: 0.5, sprint: false, timeout: Ticks(20) }]);
    let view = still();
    for _ in 0..20 {
        player.tick(&view);
        assert_eq!(player.outcome(), None);
    }
    let frame = player.tick(&view);
    assert!(frame.keys.is_empty());
    assert_eq!(player.outcome(), Some(StepOutcome::GoalTimedOut { step: 0, view: view.clone() }));
    assert!(player.tick(&view).keys.is_empty());
}

#[test]
fn walk_until_dimension_ends_on_a_dimension_change() {
    let mut player = player(vec![Step::WalkUntilDimension { dimension: None, timeout: Ticks(600) }]);
    let mut view = still();
    for _ in 0..3 {
        assert!(player.tick(&view).keys.contains(InputKey::Forward));
    }
    view.dimension = Some(1);
    assert!(player.tick(&view).keys.is_empty());
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn walk_until_dimension_waits_through_an_unknown_dimension() {
    for target in [None, Some(DimensionName::Overworld)] {
        let mut player = player(vec![Step::WalkUntilDimension { dimension: target, timeout: Ticks(600) }]);
        let mut view = still();
        view.dimension = Some(1);
        assert!(player.tick(&view).keys.contains(InputKey::Forward));
        view.dimension = None;
        for _ in 0..5 {
            assert!(player.tick(&view).keys.contains(InputKey::Forward), "{target:?}");
            assert_eq!(player.outcome(), None, "{target:?}");
        }
        view.dimension = Some(0);
        assert!(player.tick(&view).keys.is_empty(), "{target:?}");
        assert_eq!(player.outcome(), Some(StepOutcome::Done), "{target:?}");
    }
}

#[test]
fn walk_until_dimension_takes_its_start_from_the_first_known_dimension() {
    let mut player = player(vec![Step::WalkUntilDimension { dimension: None, timeout: Ticks(600) }]);
    let mut view = still();
    view.dimension = None;
    player.tick(&view);
    view.dimension = Some(1);
    assert!(player.tick(&view).keys.contains(InputKey::Forward));
    assert_eq!(player.outcome(), None);
    view.dimension = Some(2);
    assert!(player.tick(&view).keys.is_empty());
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn walk_until_dimension_to_a_named_dimension_ignores_other_changes() {
    let mut player =
        player(vec![Step::WalkUntilDimension { dimension: Some(DimensionName::Nether), timeout: Ticks(600) }]);
    let mut view = still();
    player.tick(&view);
    view.dimension = Some(2);
    assert!(player.tick(&view).keys.contains(InputKey::Forward));
    assert_eq!(player.outcome(), None);
    view.dimension = Some(1);
    assert!(player.tick(&view).keys.is_empty());
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn repeat_count_runs_the_block_n_times() {
    let mut player = player(vec![Step::Repeat {
        count: Some(3),
        timeout: None,
        steps: vec![mv(&[InputKey::Forward], 2), mv(&[InputKey::Back], 1)],
    }]);
    let frames: Vec<InputFrame> = (0..9).map(|_| player.tick(&still())).collect();
    let backs = frames.iter().filter(|frame| frame.keys.contains(InputKey::Back)).count();
    let forwards = frames.iter().filter(|frame| frame.keys.contains(InputKey::Forward)).count();
    assert_eq!((forwards, backs), (6, 3));
    assert_eq!(player.outcome(), None);
    assert!(player.tick(&still()).keys.is_empty());
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn uncounted_repeat_ends_at_its_timeout_as_done() {
    let mut player =
        player(vec![Step::Repeat { count: None, timeout: Some(Ticks(10)), steps: vec![mv(&[InputKey::Forward], 3)] }]);
    for tick in 0..10 {
        assert!(player.tick(&still()).keys.contains(InputKey::Forward), "tick {tick}");
    }
    assert!(player.tick(&still()).keys.is_empty());
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn pointer_hold_holds_for_its_ticks_then_releases() {
    let mut player =
        player(vec![Step::Pointer { action: PointerAction::Use, hold_ticks: 3 }, Step::Wait { ticks: Ticks(2) }]);
    let frames: Vec<InputFrame> = (0..5).map(|_| player.tick(&still())).collect();
    assert_eq!(frames[0].pointer.as_slice(), &[PointerOp::Press(PointerAction::Use)]);
    assert!(frames[1].pointer.is_empty());
    assert!(frames[2].pointer.is_empty());
    assert_eq!(frames[3].pointer.as_slice(), &[PointerOp::Release(PointerAction::Use)]);
    assert!(frames[4].pointer.is_empty());
}

#[test]
fn pointer_click_is_down_then_up_on_the_next_tick() {
    let mut player =
        player(vec![Step::Pointer { action: PointerAction::Attack, hold_ticks: 0 }, Step::Wait { ticks: Ticks(2) }]);
    assert_eq!(player.tick(&still()).pointer.as_slice(), &[PointerOp::Press(PointerAction::Attack)]);
    assert_eq!(player.tick(&still()).pointer.as_slice(), &[PointerOp::Release(PointerAction::Attack)]);
}

#[test]
fn a_repeat_timeout_releases_a_held_pointer() {
    let mut player = player(vec![Step::Repeat {
        count: None,
        timeout: Some(Ticks(2)),
        steps: vec![Step::Pointer { action: PointerAction::Use, hold_ticks: 10 }],
    }]);
    player.tick(&still());
    player.tick(&still());
    assert_eq!(player.tick(&still()).pointer.as_slice(), &[PointerOp::Release(PointerAction::Use)]);
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn join_waits_for_the_world_and_leave_waits_for_the_title() {
    let mut player = player(vec![
        Step::JoinServer {
            host: "127.0.0.1".to_string(),
            address: Ipv4Addr::LOCALHOST,
            port: 19132,
            timeout: Ticks(1200),
        },
        Step::LeaveWorld { timeout: Ticks(1200) },
    ]);
    let mut view = PlayerView { screen: ScreenKind::Title, ..still() };
    let target =
        Some(Arc::new(JoinTarget { host: "127.0.0.1".to_string(), address: Ipv4Addr::LOCALHOST, port: 19132 }));
    assert_eq!(player.tick(&view).join, target);
    view.screen = ScreenKind::Loading;
    assert_eq!(player.tick(&view).join, target);
    view.screen = ScreenKind::InWorld;
    let frame = player.tick(&view);
    assert_eq!(frame.join, None);
    assert!(frame.leave);
    view.screen = ScreenKind::Title;
    assert!(!player.tick(&view).leave);
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn a_join_that_starts_in_a_world_leaves_first_then_joins() {
    let mut player = player(vec![
        Step::JoinServer {
            host: "play.example.net".to_string(),
            address: Ipv4Addr::new(10, 0, 0, 2),
            port: 19133,
            timeout: Ticks(1200),
        },
        Step::Wait { ticks: Ticks(5) },
    ]);
    let target = Some(Arc::new(JoinTarget {
        host: "play.example.net".to_string(),
        address: Ipv4Addr::new(10, 0, 0, 2),
        port: 19133,
    }));
    let mut view = still();
    for _ in 0..2 {
        let frame = player.tick(&view);
        assert!(frame.leave);
        assert_eq!(frame.join, None);
    }
    view.screen = ScreenKind::Loading;
    let frame = player.tick(&view);
    assert!(frame.leave);
    assert_eq!(frame.join, None);
    view.screen = ScreenKind::Title;
    let frame = player.tick(&view);
    assert!(!frame.leave);
    assert_eq!(frame.join, target);
    view.screen = ScreenKind::Loading;
    assert_eq!(player.tick(&view).join, target);
    assert_eq!(player.current().step, 0);
    view.screen = ScreenKind::InWorld;
    let frame = player.tick(&view);
    assert_eq!(frame.join, None);
    assert!(!frame.leave);
    assert_eq!(player.current().step, 1);
}

#[test]
fn the_cursor_names_the_step_and_the_repeat_passes() {
    let mut player = player(vec![
        Step::Wait { ticks: Ticks(1) },
        Step::Repeat { count: Some(2), timeout: None, steps: vec![Step::Wait { ticks: Ticks(1) }, mv(&[], 2)] },
    ]);
    player.tick(&still());
    player.tick(&still());
    player.tick(&still());
    let cursor = player.current();
    assert_eq!(cursor.step, 3);
    assert_eq!(cursor.repeat_passes(), &[0]);
    assert_eq!(cursor.ticks_in_step, 1);
    player.tick(&still());
    player.tick(&still());
    let cursor = player.current();
    assert_eq!(cursor.step, 2);
    assert_eq!(cursor.repeat_passes(), &[1]);
}

#[test]
fn the_last_step_gives_done_and_an_empty_frame() {
    let mut player = player(vec![Step::Hotbar { slot: 3 }, mv(&[InputKey::Forward], 2)]);
    assert_eq!(player.tick(&still()).hotbar, Some(3));
    assert_eq!(player.tick(&still()).hotbar, None);
    player.tick(&still());
    assert_eq!(player.outcome(), None);
    let frame = player.tick(&still());
    assert_eq!(frame, InputFrame::default());
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}

#[test]
fn a_command_is_in_the_frame_of_its_first_tick_only() {
    let text: Arc<str> = Arc::from("/tp @s 1 2 3");
    let mut player = player(vec![Step::Command { text: Arc::clone(&text) }, mv(&[InputKey::Forward], 2)]);
    let first = player.tick(&still());
    assert_eq!(first.command.as_deref(), Some("/tp @s 1 2 3"));
    // the frame shares the step's text
    assert!(Arc::ptr_eq(first.command.as_ref().unwrap(), &text));
    let second = player.tick(&still());
    assert_eq!(second.command, None);
    assert!(second.keys.contains(InputKey::Forward));
    assert_eq!(player.current().ticks_in_step, 1);
}

#[test]
fn leave_world_from_a_world_under_a_menu_leaves() {
    let mut player = player(vec![Step::LeaveWorld { timeout: Ticks(1200) }]);
    let mut view = PlayerView { screen: ScreenKind::Other("ui://./hbui/index.html".to_string()), ..still() };
    for _ in 0..3 {
        assert!(player.tick(&view).leave);
        assert_eq!(player.outcome(), None);
    }
    view.screen = ScreenKind::Title;
    assert!(!player.tick(&view).leave);
    assert_eq!(player.outcome(), Some(StepOutcome::Done));
}
