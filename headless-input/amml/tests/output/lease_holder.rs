use alaydriem_headless_input::errors::LeaseError;
use alaydriem_headless_input::output::LeaseHolder;

use crate::support::fake_manager::{Call, FakeManager};

#[test]
fn a_refused_lease_starts_nothing() {
    let port = FakeManager::held_by("@alaydriem/dejavu");
    let mut lease = LeaseHolder::new();

    let refused = lease.begin(&port).unwrap_err();

    assert_eq!(refused, LeaseError::Refused { holder: "@alaydriem/dejavu".to_string() });
    assert!(refused.to_string().contains("@alaydriem/dejavu"));
    assert!(!lease.is_held());
    lease.end(&port);
    assert_eq!(port.calls(), vec![Call::Acquire]);
}

#[test]
fn a_held_lease_is_asked_for_once_and_given_back_once() {
    let port = FakeManager::new();
    let mut lease = LeaseHolder::new();

    lease.begin(&port).unwrap();
    lease.begin(&port).unwrap();
    assert!(lease.is_held());
    lease.end(&port);
    lease.end(&port);

    assert_eq!(port.calls(), vec![Call::Acquire, Call::Release]);
    assert!(!lease.is_held());
}
