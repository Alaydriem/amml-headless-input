use alaydriem_headless_input::errors::OutputError;
use alaydriem_headless_input::output::{InputOutput, LeaseHolder};
use amm_client::link::PointerOp as ManagerPointerOp;
use amml_api::{Look, MouseButton as PointerButton, MoveIntent};
use headless_input_core::control::{InputKey, PointerAction, TapKey};
use headless_input_core::player::{InputFrame, KeySet, PlayerView, PointerOp, ScreenKind};
use headless_input_core::run::{Command, EndReason, LiveAction, RunDefaults, RunSupervisor};
use headless_input_core::structs::SystemClock;

use crate::support::fake_manager::{Call, FakeManager};

fn keys(held: &[InputKey]) -> KeySet {
    let mut set = KeySet::new();
    for key in held {
        set.insert(*key);
    }
    set
}

fn frame_with(held: &[InputKey]) -> InputFrame {
    InputFrame { keys: keys(held), ..InputFrame::default() }
}

fn button(button: PointerButton, down: bool) -> Call {
    Call::Pointer(ManagerPointerOp::Button { button, down })
}

fn walk(forward: f32, left: f32) -> Call {
    Call::Move(MoveIntent { forward, left, ..MoveIntent::default() })
}

fn still() -> Call {
    Call::Move(MoveIntent::default())
}

fn view() -> PlayerView {
    PlayerView { position: [0.0; 3], yaw: 0.0, pitch: 0.0, dimension: Some(0), screen: ScreenKind::InWorld }
}

#[test]
fn a_move_is_sent_only_when_the_movement_keys_change() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    let forward = frame_with(&[InputKey::Forward]);

    output.apply(&forward, &port, None).unwrap();
    output.apply(&forward, &port, None).unwrap();
    assert_eq!(port.calls(), vec![walk(1.0, 0.0)]);

    output.apply(&InputFrame::default(), &port, None).unwrap();
    assert_eq!(port.calls(), vec![walk(1.0, 0.0), still()]);
}

#[test]
fn the_movement_keys_become_one_move() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    let all = [InputKey::Forward, InputKey::Left, InputKey::Jump, InputKey::Sneak, InputKey::Sprint];
    output.apply(&frame_with(&all), &port, None).unwrap();
    output.apply(&frame_with(&[InputKey::Back, InputKey::Right]), &port, None).unwrap();
    output.apply(&frame_with(&[InputKey::Forward, InputKey::Back, InputKey::Left]), &port, None).unwrap();

    assert_eq!(
        port.calls(),
        vec![
            Call::Move(MoveIntent { forward: 1.0, left: 1.0, jump: true, sneak: true, sprint: true }),
            walk(-1.0, -1.0),
            walk(0.0, 1.0),
        ]
    );
}

#[test]
fn a_manager_without_the_move_command_gets_the_movement_keys_for_the_rest_of_the_run() {
    let port = FakeManager::without_move();
    let mut output = InputOutput::new();

    output.apply(&frame_with(&[InputKey::Forward]), &port, None).unwrap();
    assert_eq!(port.calls(), vec![walk(1.0, 0.0), Call::Key(0x57, true)]);
    port.clear();

    output.apply(&frame_with(&[InputKey::Forward, InputKey::Jump]), &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x20, true)]);
    port.clear();

    output.release_all(&port);
    assert_eq!(port.calls(), vec![Call::Key(0x57, false), Call::Key(0x20, false)]);
}

#[test]
fn a_move_the_manager_is_not_ready_for_ends_the_run_with_its_own_reason() {
    let port = FakeManager::new();
    let mut lease = LeaseHolder::new();
    let mut output = InputOutput::new();
    let mut supervisor = RunSupervisor::new(RunDefaults::default(), SystemClock::new());
    lease.begin(&port).unwrap();
    port.fail_moves(OutputError::NotReady);
    supervisor.apply(Command::Live(LiveAction::Hold { keys: keys(&[InputKey::Forward]), ticks: None })).unwrap();

    let refused = output.apply(&supervisor.on_tick(&view()), &port, None).unwrap_err();
    assert_eq!(refused, OutputError::NotReady);
    assert_ne!(refused.why(), OutputError::Refused.why());
    supervisor.abort(refused.why());

    let outcome = supervisor.on_frame(&view(), false, true);
    assert_eq!(outcome.end, Some(EndReason::Aborted { why: refused.why().into() }));
    port.clear();
    output.end(&port, &mut lease);
    assert_eq!(port.calls(), vec![Call::Release], "nothing was taken, so nothing is cleared");
}

#[test]
fn a_key_down_into_a_full_manager_queue_is_busy() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    port.fail_keys(OutputError::Busy);

    let full = output.apply(&tap("E"), &port, None);

    assert_eq!(full, Err(OutputError::Busy));
    port.clear();
    output.release_all(&port);
    assert!(port.calls().is_empty(), "a refused down leaves nothing to let go");
}

#[test]
fn release_all_sends_ups_for_every_held_key_and_button() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    let mut frame = frame_with(&[InputKey::Forward, InputKey::Jump, InputKey::Sprint]);
    frame.pointer.push(PointerOp::Press(PointerAction::Attack));
    frame.pointer.push(PointerOp::Press(PointerAction::Use));
    frame.hotbar = Some(5);
    output.apply(&frame, &port, None).unwrap();
    port.clear();

    output.release_all(&port);

    let calls = port.calls();
    assert_eq!(calls.len(), 3);
    for up in [still(), button(PointerButton::Left, false), button(PointerButton::Right, false)] {
        assert!(calls.contains(&up), "{up:?} not sent in {calls:?}");
    }

    port.clear();
    output.release_all(&port);
    assert!(port.calls().is_empty());
}

#[test]
fn end_releases_inputs_before_the_lease() {
    let port = FakeManager::new();
    let mut lease = LeaseHolder::new();
    let mut output = InputOutput::new();
    lease.begin(&port).unwrap();
    let mut frame = frame_with(&[InputKey::Back]);
    frame.pointer.push(PointerOp::Press(PointerAction::Use));
    output.apply(&frame, &port, None).unwrap();
    port.clear();

    output.end(&port, &mut lease);

    assert_eq!(port.calls(), vec![still(), button(PointerButton::Right, false), Call::Release]);
    assert!(!lease.is_held());
}

#[test]
fn hotbar_slot_3_selects_hud_slot_2_once() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&InputFrame { hotbar: Some(3), ..InputFrame::default() }, &port, None).unwrap();
    output.apply(&InputFrame::default(), &port, None).unwrap();

    assert_eq!(port.calls(), vec![Call::Hotbar(2)]);
}

#[test]
fn hotbar_slots_1_and_9_are_the_ends_of_the_hud() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&InputFrame { hotbar: Some(1), ..InputFrame::default() }, &port, None).unwrap();
    output.apply(&InputFrame { hotbar: Some(9), ..InputFrame::default() }, &port, None).unwrap();

    assert_eq!(port.calls(), vec![Call::Hotbar(0), Call::Hotbar(8)]);
}

#[test]
fn a_hotbar_slot_outside_1_to_9_sends_nothing() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    for slot in [0, 10, 255] {
        let frame = InputFrame { hotbar: Some(slot), ..InputFrame::default() };
        output.apply(&frame, &port, None).unwrap();
    }

    assert!(port.calls().is_empty());
}

#[test]
fn a_press_of_a_held_button_sends_nothing_and_a_release_lets_it_go() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    let mut press = InputFrame::default();
    press.pointer.push(PointerOp::Press(PointerAction::Attack));
    let mut release = InputFrame::default();
    release.pointer.push(PointerOp::Release(PointerAction::Attack));

    output.apply(&press, &port, None).unwrap();
    output.apply(&press, &port, None).unwrap();
    output.apply(&release, &port, None).unwrap();
    output.apply(&release, &port, None).unwrap();

    assert_eq!(port.calls(), vec![button(PointerButton::Left, true), button(PointerButton::Left, false)]);
}

#[test]
fn a_scroll_is_one_wheel_turn_and_a_zero_scroll_sends_nothing() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    let mut frame = InputFrame::default();
    frame.pointer.push(PointerOp::Scroll(-70));
    frame.pointer.push(PointerOp::Scroll(0));

    output.apply(&frame, &port, None).unwrap();

    assert_eq!(port.calls(), vec![Call::Pointer(ManagerPointerOp::Wheel { notches: -70.0 })]);
}

#[test]
fn forget_drops_the_record_without_sending() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    output.apply(&frame_with(&[InputKey::Sneak]), &port, None).unwrap();
    port.clear();

    output.forget();
    output.release_all(&port);

    assert!(port.calls().is_empty());
}

#[test]
fn a_refused_move_ends_the_run_aborted() {
    let port = FakeManager::new();
    let mut lease = LeaseHolder::new();
    let mut output = InputOutput::new();
    let mut supervisor = RunSupervisor::new(RunDefaults::default(), SystemClock::new());
    lease.begin(&port).unwrap();
    supervisor.apply(Command::Live(LiveAction::Hold { keys: keys(&[InputKey::Forward]), ticks: None })).unwrap();
    let first = supervisor.on_tick(&view());
    output.apply(&first, &port, None).unwrap();

    port.open_panel();
    supervisor.apply(Command::Live(LiveAction::Hold { keys: keys(&[InputKey::Left]), ticks: None })).unwrap();
    let second = supervisor.on_tick(&view());
    let refused = output.apply(&second, &port, None).unwrap_err();
    assert_eq!(refused, OutputError::Refused);
    supervisor.abort(refused.why());

    let outcome = supervisor.on_frame(&view(), false, true);
    assert_eq!(outcome.end, Some(EndReason::Aborted { why: refused.why().into() }));
    port.clear();
    let _ = output.apply(&outcome.frame.unwrap(), &port, None);
    output.end(&port, &mut lease);
    // the clear is taken even while the panel is open
    assert_eq!(port.calls(), vec![still(), Call::Release]);
}

#[test]
fn another_mod_taking_the_lease_mid_run_still_gets_every_held_key_released_before_the_lease() {
    let port = FakeManager::new();
    let mut lease = LeaseHolder::new();
    let mut output = InputOutput::new();
    lease.begin(&port).unwrap();
    let mut frame = frame_with(&[InputKey::Forward, InputKey::Sprint]);
    frame.pointer.push(PointerOp::Press(PointerAction::Attack));
    output.apply(&frame, &port, None).unwrap();

    port.take_lease();
    let refused = output.apply(&frame_with(&[InputKey::Forward, InputKey::Sprint, InputKey::Jump]), &port, None);
    assert_eq!(refused, Err(OutputError::Refused));
    port.clear();
    output.end(&port, &mut lease);

    assert_eq!(port.calls(), vec![still(), button(PointerButton::Left, false), Call::Release]);
}

fn turn(yaw: f32, pitch: f32) -> Call {
    Call::Pointer(ManagerPointerOp::Turn(Look { yaw, pitch }))
}

fn look(yaw: f32, pitch: f32) -> InputFrame {
    InputFrame { rotation: Some((yaw, pitch)), ..InputFrame::default() }
}

#[test]
fn a_rotation_sends_the_difference_from_the_last_commanded_look() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&look(30.0, 10.0), &port, Some((0.0, 0.0))).unwrap();
    // the read look is far from the commanded one, as when the camera has not caught up
    output.apply(&look(45.0, 10.0), &port, Some((-120.0, 70.0))).unwrap();
    output.apply(&look(45.0, -5.0), &port, Some((-120.0, 70.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(30.0, 10.0), turn(15.0, 0.0), turn(0.0, -15.0)]);
}

#[test]
fn the_first_rotation_of_a_step_starts_from_the_read_look() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&look(110.0, 20.0), &port, Some((100.0, 20.0))).unwrap();
    assert_eq!(port.calls(), vec![turn(10.0, 0.0)]);
    port.clear();

    // a frame with no rotation ends the step: the next rotation starts from the read look again
    output.apply(&InputFrame::default(), &port, Some((110.0, 20.0))).unwrap();
    output.apply(&look(0.0, 20.0), &port, Some((-20.0, 25.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(20.0, -5.0)]);
}

#[test]
fn a_yaw_difference_across_180_takes_the_short_way() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&look(-170.0, 0.0), &port, Some((170.0, 0.0))).unwrap();
    output.apply(&look(170.0, 0.0), &port, Some((0.0, 0.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(20.0, 0.0), turn(-20.0, 0.0)]);
}

#[test]
fn the_pitch_is_held_to_straight_up_and_down() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&look(0.0, 120.0), &port, Some((0.0, 80.0))).unwrap();
    output.apply(&look(0.0, 95.0), &port, Some((0.0, 80.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(0.0, 10.0)]);
}

#[test]
fn a_rotation_that_cannot_be_sent_sends_nothing() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    for rotation in [(f32::NAN, 10.0), (45.0, f32::INFINITY)] {
        output.apply(&look(rotation.0, rotation.1), &port, Some((0.0, 0.0))).unwrap();
    }
    // no read look and nothing commanded before
    output.apply(&look(45.0, 10.0), &port, None).unwrap();
    // a rotation equal to the look turns nothing
    output.apply(&look(0.0, 0.0), &port, Some((0.0, 0.0))).unwrap();

    assert!(port.calls().is_empty());
}

#[test]
fn a_rotation_is_not_sent_while_the_panel_is_open() {
    let port = FakeManager::new();
    port.open_panel();
    let mut output = InputOutput::new();

    output.apply(&look(45.0, 10.0), &port, Some((0.0, 0.0))).unwrap();

    assert!(port.calls().is_empty());
}

#[test]
fn a_turn_the_manager_refuses_is_an_error() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    port.take_lease();

    let refused = output.apply(&look(45.0, 0.0), &port, Some((0.0, 0.0)));
    assert_eq!(refused, Err(OutputError::Refused));
}

#[test]
fn forget_drops_the_commanded_look() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    output.apply(&look(30.0, 0.0), &port, Some((0.0, 0.0))).unwrap();
    port.clear();

    output.forget();
    output.apply(&look(30.0, 0.0), &port, Some((20.0, 0.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(10.0, 0.0)]);
}

fn tap(name: &str) -> InputFrame {
    InputFrame { tap: Some(TapKey::parse(name).unwrap()), ..InputFrame::default() }
}

#[test]
fn a_press_taps_its_key_down_then_up_on_the_next_frame() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&tap("F5"), &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x74, true)]);

    output.apply(&InputFrame::default(), &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x74, true), Call::Key(0x74, false)]);
}

#[test]
fn a_press_and_a_hotbar_select_in_one_frame_both_reach_the_game() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&InputFrame { hotbar: Some(2), ..tap("E") }, &port, None).unwrap();
    let downs = port.calls();
    assert_eq!(downs, vec![Call::Hotbar(1), Call::Key(0x45, true)]);
    port.clear();

    output.apply(&InputFrame::default(), &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x45, false)]);
}

#[test]
fn the_same_key_pressed_twice_in_a_row_goes_up_then_down() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&tap("Esc"), &port, None).unwrap();
    port.clear();
    output.apply(&tap("Esc"), &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x1B, false), Call::Key(0x1B, true)]);
}

#[test]
fn release_all_lets_go_of_a_pressed_key() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&tap("Esc"), &port, None).unwrap();
    port.clear();
    output.release_all(&port);
    assert_eq!(port.calls(), vec![Call::Key(0x1B, false)]);
}
