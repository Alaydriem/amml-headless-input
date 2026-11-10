use crate::control::InputKey;

// A set of held movement keys, one bit per key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeySet(u8);

impl KeySet {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn insert(&mut self, key: InputKey) {
        self.0 |= Self::bit(key);
    }

    pub fn remove(&mut self, key: InputKey) {
        self.0 &= !Self::bit(key);
    }

    pub fn contains(self, key: InputKey) -> bool {
        self.0 & Self::bit(key) != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn bit(key: InputKey) -> u8 {
        match key {
            InputKey::Forward => 1,
            InputKey::Back => 1 << 1,
            InputKey::Left => 1 << 2,
            InputKey::Right => 1 << 3,
            InputKey::Jump => 1 << 4,
            InputKey::Sneak => 1 << 5,
            InputKey::Sprint => 1 << 6,
        }
    }
}
