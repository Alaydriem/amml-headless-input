use std::sync::Arc;

use alaydriem_headless_input::command::CommandService;
use alaydriem_headless_input::errors::CommandError;

use crate::support::fake_command_client::FakeCommandClient;

fn text(command: &str) -> Arc<str> {
    Arc::from(command)
}

#[test]
fn a_request_is_sent_once_on_the_next_pump() {
    let client = FakeCommandClient::new();
    let mut service = CommandService::new();
    assert!(service.request(&text("/tp @s 1 2 3")));

    assert_eq!(service.pump(&client), Ok(()));
    assert_eq!(service.pump(&client), Ok(()));

    assert_eq!(client.sent(), vec!["/tp @s 1 2 3".to_string()]);
    assert!(service.is_empty());
}

#[test]
fn commands_are_sent_in_the_order_they_were_asked() {
    let client = FakeCommandClient::new();
    let mut service = CommandService::new();
    service.request(&text("/a"));
    service.request(&text("/b"));

    assert_eq!(service.pump(&client), Ok(()));

    assert_eq!(client.sent(), vec!["/a".to_string(), "/b".to_string()]);
}

#[test]
fn no_local_player_sends_nothing_and_drops_the_request() {
    let client = FakeCommandClient::new();
    client.set_player(false);
    let mut service = CommandService::new();
    service.request(&text("/tp @s 1 2 3"));

    assert_eq!(service.pump(&client), Err(CommandError::NoPlayer));

    client.set_player(true);
    assert_eq!(service.pump(&client), Ok(()));
    assert!(client.sent().is_empty());
}

#[test]
fn a_command_the_game_did_not_take_ends_the_pump_and_drops_the_rest() {
    let client = FakeCommandClient::new();
    client.set_takes(false);
    let mut service = CommandService::new();
    service.request(&text("/a"));
    service.request(&text("/b"));

    assert_eq!(service.pump(&client), Err(CommandError::NotSent));

    client.set_takes(true);
    assert_eq!(service.pump(&client), Ok(()));
    assert!(client.sent().is_empty());
}

#[test]
fn a_request_past_the_capacity_is_dropped_and_the_earlier_ones_stay() {
    let client = FakeCommandClient::new();
    let mut service = CommandService::new();
    for index in 0..CommandService::CAPACITY {
        assert!(service.request(&text(&format!("/{index}"))));
    }

    assert!(!service.request(&text("/over")));
    assert_eq!(service.pump(&client), Ok(()));

    let sent = client.sent();
    assert_eq!(sent.len(), CommandService::CAPACITY);
    assert_eq!(sent.first().map(String::as_str), Some("/0"));
    assert!(!sent.iter().any(|command| command == "/over"));
}

#[test]
fn clearing_drops_the_waiting_requests() {
    let client = FakeCommandClient::new();
    let mut service = CommandService::new();
    service.request(&text("/a"));
    service.request(&text("/b"));

    service.clear();

    assert!(service.is_empty());
    assert_eq!(service.pump(&client), Ok(()));
    assert!(client.sent().is_empty());
    assert!(service.request(&text("/c")));
}
