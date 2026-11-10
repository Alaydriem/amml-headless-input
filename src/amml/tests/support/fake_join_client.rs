use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use alaydriem_headless_input::join::JoinClient;
use headless_input_core::structs::JoinTarget;

// a game client stand-in: the test sets the screen and the flags, and the fake records each join and leave call
pub struct FakeJoinClient {
    screen: RefCell<String>,
    primary_ready: Cell<bool>,
    in_game: Cell<bool>,
    // the answers of the next join calls; an empty queue takes the join
    answers: RefCell<VecDeque<bool>>,
    // (address, port, host) of each join call
    joins: RefCell<Vec<(String, u16, String)>>,
    leaves: Cell<u32>,
    // the answers of the next servers tab calls; an empty queue opens the tab
    tab_answers: RefCell<VecDeque<bool>>,
    // what an empty queue answers
    tab_opens: Cell<bool>,
    tabs: Cell<u32>,
    // the order of the tab and join calls
    events: RefCell<Vec<&'static str>>,
    // the result code the game's callback left, until it is taken or cleared
    result: Cell<Option<i32>>,
}

impl FakeJoinClient {
    pub fn new(screen: &str) -> Self {
        Self {
            screen: RefCell::new(screen.to_string()),
            primary_ready: Cell::new(true),
            in_game: Cell::new(false),
            answers: RefCell::new(VecDeque::new()),
            joins: RefCell::new(Vec::new()),
            leaves: Cell::new(0),
            tab_answers: RefCell::new(VecDeque::new()),
            tab_opens: Cell::new(true),
            tabs: Cell::new(0),
            events: RefCell::new(Vec::new()),
            result: Cell::new(None),
        }
    }

    pub fn in_world() -> Self {
        let client = Self::new("hud_screen");
        client.in_game.set(true);
        client
    }

    pub fn show(&self, screen: &str) {
        *self.screen.borrow_mut() = screen.to_string();
    }

    pub fn set_in_game(&self, in_game: bool) {
        self.in_game.set(in_game);
    }

    pub fn set_primary_ready(&self, ready: bool) {
        self.primary_ready.set(ready);
    }

    pub fn answer_next(&self, taken: bool) {
        self.answers.borrow_mut().push_back(taken);
    }

    pub fn joins(&self) -> Vec<(String, u16, String)> {
        self.joins.borrow().clone()
    }

    pub fn leaves(&self) -> u32 {
        self.leaves.get()
    }

    pub fn answer_tab_next(&self, opened: bool) {
        self.tab_answers.borrow_mut().push_back(opened);
    }

    pub fn set_tab_opens(&self, opens: bool) {
        self.tab_opens.set(opens);
    }

    pub fn tabs(&self) -> u32 {
        self.tabs.get()
    }

    // the game's callback answers with `code`
    pub fn answer_with(&self, code: i32) {
        self.result.set(Some(code));
    }

    pub fn events(&self) -> Vec<&'static str> {
        self.events.borrow().clone()
    }
}

impl JoinClient for FakeJoinClient {
    fn is_primary_ready(&self) -> bool {
        self.primary_ready.get()
    }

    fn is_in_game(&self) -> bool {
        self.in_game.get()
    }

    fn top_screen_name(&self) -> String {
        self.screen.borrow().clone()
    }

    fn request_leave_game(&self) {
        self.leaves.set(self.leaves.get() + 1);
    }

    fn open_servers_tab(&self) -> bool {
        self.tabs.set(self.tabs.get() + 1);
        self.events.borrow_mut().push("tab");
        self.tab_answers.borrow_mut().pop_front().unwrap_or(self.tab_opens.get())
    }

    fn take_join_result(&self) -> Option<i32> {
        self.result.take()
    }

    fn clear_join_result(&self) {
        self.result.set(None);
    }

    fn join_server(&self, target: &JoinTarget) -> bool {
        self.events.borrow_mut().push("join");
        self.joins.borrow_mut().push((target.address.to_string(), target.port, target.host.clone()));
        self.answers.borrow_mut().pop_front().unwrap_or(true)
    }
}
