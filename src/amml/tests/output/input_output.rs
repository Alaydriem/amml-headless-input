use alaydriem_headless_input::errors::OutputError;
use alaydriem_headless_input::output::{InputOutput, LeaseHolder};
use amm_core::structs::input::{PointerButton, PointerOp as ManagerPointerOp, TurnAxis};
use headless_input_core::control::{InputKey, PointerAction};
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

fn view() -> PlayerView {
    PlayerView { position: [0.0; 3], yaw: 0.0, pitch: 0.0, dimension: Some(0), screen: ScreenKind::InWorld }
}

#[test]
fn only_changed_keys_are_sent() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    let forward = frame_with(&[InputKey::Forward]);

    output.apply(&forward, &port, None).unwrap();
    output.apply(&forward, &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x57, true)]);

    output.apply(&InputFrame::default(), &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x57, true), Call::Key(0x57, false)]);
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
    assert_eq!(calls.len(), 6);
    for up in [
        Call::Key(0x57, false),
        Call::Key(0x20, false),
        Call::Key(0x11, false),
        Call::Key(0x35, false),
        button(PointerButton::Left, false),
        button(PointerButton::Right, false),
    ] {
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

    assert_eq!(port.calls(), vec![Call::Key(0x53, false), button(PointerButton::Right, false), Call::Release]);
    assert!(!lease.is_held());
}

#[test]
fn hotbar_slot_3_presses_and_releases_the_3_key() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&InputFrame { hotbar: Some(3), ..InputFrame::default() }, &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x33, true)]);

    output.apply(&InputFrame::default(), &port, None).unwrap();
    assert_eq!(port.calls(), vec![Call::Key(0x33, true), Call::Key(0x33, false)]);
}

#[test]
fn a_new_hotbar_slot_releases_the_last_one_first() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&InputFrame { hotbar: Some(1), ..InputFrame::default() }, &port, None).unwrap();
    output.apply(&InputFrame { hotbar: Some(9), ..InputFrame::default() }, &port, None).unwrap();

    assert_eq!(port.calls(), vec![Call::Key(0x31, true), Call::Key(0x31, false), Call::Key(0x39, true)]);
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
fn a_scroll_past_the_wheel_limit_is_sent_in_turns() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();
    let mut frame = InputFrame::default();
    frame.pointer.push(PointerOp::Scroll(-70));
    frame.pointer.push(PointerOp::Scroll(0));

    output.apply(&frame, &port, None).unwrap();

    let wheel = |steps| Call::Pointer(ManagerPointerOp::Wheel { steps });
    assert_eq!(port.calls(), vec![wheel(-32), wheel(-32), wheel(-6)]);
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
fn a_refused_send_key_ends_the_run_aborted() {
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
    assert_eq!(port.calls(), vec![Call::Key(0x57, false), Call::Key(0x57, false), Call::Release]);
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

    assert_eq!(
        port.calls(),
        vec![Call::Key(0x57, false), Call::Key(0x11, false), button(PointerButton::Left, false), Call::Release]
    );
}

fn turn(axis: TurnAxis, millidegrees: i32) -> Call {
    Call::Pointer(ManagerPointerOp::Turn { axis, millidegrees })
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

    assert_eq!(
        port.calls(),
        vec![
            turn(TurnAxis::Yaw, 30_000),
            turn(TurnAxis::Pitch, -10_000),
            turn(TurnAxis::Yaw, 15_000),
            turn(TurnAxis::Pitch, 15_000),
        ]
    );
}

#[test]
fn the_first_rotation_of_a_step_starts_from_the_read_look() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&look(110.0, 20.0), &port, Some((100.0, 20.0))).unwrap();
    assert_eq!(port.calls(), vec![turn(TurnAxis::Yaw, 10_000)]);
    port.clear();

    // a frame with no rotation ends the step: the next rotation starts from the read look again
    output.apply(&InputFrame::default(), &port, Some((110.0, 20.0))).unwrap();
    output.apply(&look(0.0, 20.0), &port, Some((-20.0, 25.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(TurnAxis::Yaw, 20_000), turn(TurnAxis::Pitch, 5_000)]);
}

#[test]
fn a_yaw_difference_across_180_takes_the_short_way() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&look(-170.0, 0.0), &port, Some((170.0, 0.0))).unwrap();
    output.apply(&look(170.0, 0.0), &port, Some((0.0, 0.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(TurnAxis::Yaw, 20_000), turn(TurnAxis::Yaw, -20_000)]);
}

#[test]
fn the_pitch_is_held_to_straight_up_and_down() {
    let port = FakeManager::new();
    let mut output = InputOutput::new();

    output.apply(&look(0.0, 120.0), &port, Some((0.0, 80.0))).unwrap();
    output.apply(&look(0.0, 95.0), &port, Some((0.0, 80.0))).unwrap();

    assert_eq!(port.calls(), vec![turn(TurnAxis::Pitch, -10_000)]);
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

    assert_eq!(port.calls(), vec![turn(TurnAxis::Yaw, 10_000)]);
}
