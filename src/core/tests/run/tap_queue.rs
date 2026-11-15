use headless_input_core::control::TapKey;
use headless_input_core::run::TapQueue;

fn key(name: &str) -> TapKey {
    TapKey::parse(name).unwrap()
}

#[test]
fn presses_leave_in_the_order_they_came() {
    let mut queue = TapQueue::new();
    assert!(queue.push(key("F5")));
    assert!(queue.push(key("E")));
    assert_eq!(queue.pop(), Some(key("F5")));
    assert_eq!(queue.pop(), Some(key("E")));
    assert_eq!(queue.pop(), None);
}

#[test]
fn a_push_past_capacity_is_refused_and_the_queued_presses_stay() {
    let mut queue = TapQueue::new();
    for _ in 0..TapQueue::CAPACITY {
        assert!(queue.push(key("F5")));
    }
    assert!(!queue.push(key("E")));
    assert_eq!(queue.len(), TapQueue::CAPACITY);
    for _ in 0..TapQueue::CAPACITY {
        assert_eq!(queue.pop(), Some(key("F5")));
    }
    assert_eq!(queue.pop(), None);
}

#[test]
fn a_queue_that_wrapped_keeps_its_order() {
    let mut queue = TapQueue::new();
    for _ in 0..TapQueue::CAPACITY - 1 {
        queue.push(key("F5"));
        queue.pop();
    }
    assert!(queue.push(key("E")));
    assert!(queue.push(key("Esc")));
    assert_eq!(queue.pop(), Some(key("E")));
    assert_eq!(queue.pop(), Some(key("Esc")));
}
