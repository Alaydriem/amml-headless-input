use crate::control::TapKey;

// The presses a live session has not tapped yet, first in first out, at most CAPACITY. A push to a full queue is
// refused and the queued presses stay.
#[derive(Debug, Clone, Copy)]
pub struct TapQueue {
    keys: [Option<TapKey>; Self::CAPACITY],
    head: usize,
    len: usize,
}

impl TapQueue {
    pub const CAPACITY: usize = 8;

    pub fn new() -> Self {
        Self { keys: [None; Self::CAPACITY], head: 0, len: 0 }
    }

    // false when CAPACITY presses already wait
    pub fn push(&mut self, key: TapKey) -> bool {
        if self.len == Self::CAPACITY {
            return false;
        }
        self.keys[(self.head + self.len) % Self::CAPACITY] = Some(key);
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<TapKey> {
        if self.len == 0 {
            return None;
        }
        let key = self.keys[self.head].take();
        self.head = (self.head + 1) % Self::CAPACITY;
        self.len -= 1;
        key
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == Self::CAPACITY
    }
}

impl Default for TapQueue {
    fn default() -> Self {
        Self::new()
    }
}
