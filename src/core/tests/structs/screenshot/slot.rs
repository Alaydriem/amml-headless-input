use std::path::PathBuf;

use headless_input_core::structs::{ScreenshotRequest, ScreenshotSlot};

fn request() -> ScreenshotRequest {
    ScreenshotRequest::parse(None, None).unwrap()
}

fn path() -> PathBuf {
    PathBuf::from("shot.png")
}

fn asked(slot: &mut ScreenshotSlot, sequence: u32) -> u64 {
    let ticket = slot.queue(request(), path()).unwrap();
    let (taken, _, _) = slot.take_queued().unwrap();
    assert_eq!(taken, ticket);
    slot.asked(ticket, sequence);
    ticket
}

#[test]
fn second_request_while_one_is_in_flight_is_refused() {
    let mut slot = ScreenshotSlot::new();
    slot.queue(request(), path()).unwrap();
    assert_eq!(slot.queue(request(), path()), Err("a screenshot is already in progress"));
}

#[test]
fn written_for_the_asked_sequence_gives_the_path() {
    let mut slot = ScreenshotSlot::new();
    let ticket = asked(&mut slot, 7);
    assert_eq!(slot.outcome(ticket), None);
    slot.on_taken(7, true);
    assert_eq!(slot.outcome(ticket), None);
    slot.on_written(7, true);
    assert_eq!(slot.outcome(ticket), Some(Ok(path())));
}

#[test]
fn written_for_another_sequence_is_ignored() {
    let mut slot = ScreenshotSlot::new();
    let ticket = asked(&mut slot, 7);
    slot.on_written(8, true);
    assert_eq!(slot.outcome(ticket), None);
}

#[test]
fn taken_not_ok_fails_at_once() {
    let mut slot = ScreenshotSlot::new();
    let ticket = asked(&mut slot, 7);
    slot.on_taken(7, false);
    assert_eq!(slot.outcome(ticket), Some(Err("the manager could not take the screenshot")));
}

#[test]
fn written_not_ok_fails() {
    let mut slot = ScreenshotSlot::new();
    let ticket = asked(&mut slot, 7);
    slot.on_taken(7, true);
    slot.on_written(7, false);
    assert_eq!(slot.outcome(ticket), Some(Err("the manager could not write the screenshot")));
}

#[test]
fn refused_gives_the_reason() {
    let mut slot = ScreenshotSlot::new();
    let ticket = slot.queue(request(), path()).unwrap();
    slot.take_queued().unwrap();
    slot.refused(ticket, "not in a world");
    assert_eq!(slot.outcome(ticket), Some(Err("not in a world")));
}

#[test]
fn abandon_frees_the_slot_and_a_late_event_is_ignored() {
    let mut slot = ScreenshotSlot::new();
    let old = asked(&mut slot, 7);
    slot.abandon(old);
    let new = slot.queue(request(), path()).unwrap();
    slot.take_queued().unwrap();
    slot.asked(new, 9);
    slot.on_written(7, true);
    assert_eq!(slot.outcome(new), None);
    assert_eq!(slot.outcome(old), None);
}

#[test]
fn outcome_frees_the_slot() {
    let mut slot = ScreenshotSlot::new();
    let ticket = asked(&mut slot, 7);
    slot.on_taken(7, false);
    assert!(slot.outcome(ticket).is_some());
    assert!(slot.queue(request(), path()).is_ok());
}
