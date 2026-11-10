use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use headless_input_core::traits::Clock;

// A clock that moves only when the test advances it; clones share one time.
#[derive(Debug, Clone)]
pub struct FakeClock {
    now: Rc<Cell<Instant>>,
}

impl FakeClock {
    pub fn new() -> Self {
        Self { now: Rc::new(Cell::new(Instant::now())) }
    }

    pub fn advance(&self, by: Duration) {
        self.now.set(self.now.get() + by);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.now.get()
    }
}
