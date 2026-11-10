use std::sync::Arc;

use alaydriem_headless_input::services::{CommandInbox, CommandReply};
use headless_input_core::run::{Command, CommandQueue};
use headless_input_mcp::port::PortError;

#[test]
fn a_full_inbox_is_busy_and_keeps_its_commands() {
    let inbox = CommandInbox::new();
    for _ in 0..CommandQueue::CAPACITY {
        assert_eq!(inbox.push(Command::Stop, Arc::new(CommandReply::new())), Ok(()));
    }

    let refused = inbox.push(Command::Stop, Arc::new(CommandReply::new()));

    assert_eq!(refused, Err(PortError::Busy));
    assert_eq!(inbox.len(), CommandQueue::CAPACITY);
    let mut commands = Vec::new();
    let mut replies = Vec::new();
    inbox.drain(&mut commands, &mut replies);
    assert_eq!((commands.len(), replies.len()), (CommandQueue::CAPACITY, CommandQueue::CAPACITY));
    assert_eq!(inbox.len(), 0);
}
