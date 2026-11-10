use std::path::Path;

use alaydriem_headless_input::errors::{LeaseError, OutputError};
use alaydriem_headless_input::services::HeadlessLink;
use alaydriem_headless_input::traits::ManagerPort;
use amm_abi::types::AmmStatus;
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amm_core::structs::input::{PointerButton, PointerOp};
use amm_core::structs::lease::AcquireOutcome;
use amm_core::structs::registry::ModIndex;

fn link() -> HeadlessLink {
    HeadlessLink::new(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR)).unwrap()
}

#[test]
fn reload_reads_a_changed_setting() {
    FakeTable::set_config("mcp {\n  enabled = false\n}\n");
    let mut link = link();
    assert!(!link.config().mcp_enabled);

    FakeTable::set_config("mcp {\n  enabled = true\n}\n");
    link.reload();

    assert!(link.config().mcp_enabled);
}

#[test]
fn a_drain_reads_at_most_the_per_tick_bound_and_defers_the_rest_in_order() {
    let mut link = link();
    let bound = HeadlessLink::MAX_EVENTS_PER_TICK;
    let extra = 10;
    let event = |index: usize| ModEvent::Key { vk: index as u32, down: true, repeat: false };
    for index in 0..bound + extra {
        FakeTable::push_event(event(index));
    }

    let first = link.drain().to_vec();
    assert_eq!(first.len(), bound);
    assert_eq!(first.first().copied(), Some(event(0)));
    assert_eq!(first.last().copied(), Some(event(bound - 1)));

    let second = link.drain().to_vec();
    assert_eq!(second, (bound..bound + extra).map(event).collect::<Vec<_>>());
    assert!(link.drain().is_empty());
}

#[test]
fn a_denied_input_lease_names_the_holder() {
    FakeTable::set_acquire_outcome(Capability::Input, AcquireOutcome::Denied { holder: ModIndex(2) });
    FakeTable::with(|state| state.holder_id = Some("@alaydriem/dejavu"));
    let link = link();

    let refused = link.acquire_input();

    assert_eq!(refused, Err(LeaseError::Refused { holder: "@alaydriem/dejavu".to_string() }));
    assert_eq!(FakeTable::with(|state| state.acquired.clone()), vec![Capability::Input]);
}

#[test]
fn a_preempting_input_lease_is_held() {
    FakeTable::set_acquire_outcome(Capability::Input, AcquireOutcome::Preempted);
    let link = link();

    assert_eq!(link.acquire_input(), Ok(()));
    link.release_input();
    assert_eq!(FakeTable::with(|state| state.released.clone()), vec![Capability::Input]);
}

#[test]
fn panel_open_is_the_manager_answer() {
    let link = link();
    assert!(!link.panel_open());

    FakeTable::with(|state| state.panel_open = true);
    assert!(link.panel_open());

    FakeTable::with(|state| state.panel_open = false);
    assert!(!link.panel_open());
}

#[test]
fn a_send_the_manager_does_not_permit_is_refused_and_is_not_the_panel() {
    let link = link();
    FakeTable::with(|state| state.send_key_status = AmmStatus::NotPermitted);

    assert_eq!(link.send_key(0x57, true), Err(OutputError::Refused));
    assert!(!link.panel_open());
}

#[test]
fn a_full_pointer_queue_is_its_own_error() {
    let link = link();
    FakeTable::with(|state| state.send_pointer_status = AmmStatus::QueueFull);

    let full = link.send_pointer(PointerOp::Button { button: PointerButton::Left, down: true });

    assert_eq!(full, Err(OutputError::QueueFull));
    assert!(!link.panel_open());
}

#[test]
fn the_manager_letting_go_of_the_fake_keys_is_not_the_panel() {
    let mut link = link();
    FakeTable::push_event(ModEvent::FakeKeysReleased);

    link.drain();

    assert!(!link.panel_open());
}
