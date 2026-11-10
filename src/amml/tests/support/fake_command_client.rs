use std::cell::{Cell, RefCell};

use alaydriem_headless_input::command::CommandClient;

// a game client stand-in for commands: the test sets whether a local player exists and whether the send is taken, and
// the fake records each command it was asked to send
pub struct FakeCommandClient {
    player: Cell<bool>,
    takes: Cell<bool>,
    sent: RefCell<Vec<String>>,
}

impl FakeCommandClient {
    pub fn new() -> Self {
        Self { player: Cell::new(true), takes: Cell::new(true), sent: RefCell::new(Vec::new()) }
    }

    pub fn set_player(&self, player: bool) {
        self.player.set(player);
    }

    pub fn set_takes(&self, takes: bool) {
        self.takes.set(takes);
    }

    pub fn sent(&self) -> Vec<String> {
        self.sent.borrow().clone()
    }
}

impl CommandClient for FakeCommandClient {
    fn has_local_player(&self) -> bool {
        self.player.get()
    }

    fn send_command(&self, text: &str) -> bool {
        if !self.takes.get() {
            return false;
        }
        self.sent.borrow_mut().push(text.to_string());
        true
    }
}
