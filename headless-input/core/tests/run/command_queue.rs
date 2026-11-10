use headless_input_core::run::{Command, CommandQueue};

#[test]
fn command_queue_refuses_past_capacity() {
    let mut queue = CommandQueue::new();
    for _ in 0..CommandQueue::CAPACITY {
        assert!(queue.try_push(Command::Stop).is_ok());
    }
    let refused = queue.try_push(Command::Stop);
    assert!(matches!(refused, Err(Command::Stop)));
    assert_eq!(queue.len(), CommandQueue::CAPACITY);
    assert_eq!(CommandQueue::CAPACITY, 64);

    let mut out = Vec::new();
    queue.drain(&mut out);
    assert_eq!(out.len(), CommandQueue::CAPACITY);
    assert!(queue.is_empty());
    assert!(queue.try_push(Command::Stop).is_ok());
}
