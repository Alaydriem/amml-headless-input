use super::PointerOp;

// The pointer events of one tick, at most CAPACITY. A push past capacity is refused and the earlier events stay.
#[derive(Debug, Clone, Copy)]
pub struct PointerOps {
    ops: [PointerOp; Self::CAPACITY],
    len: usize,
}

impl PointerOps {
    pub const CAPACITY: usize = 4;

    pub fn new() -> Self {
        Self { ops: [PointerOp::Scroll(0); Self::CAPACITY], len: 0 }
    }

    // false when the tick already holds CAPACITY events
    pub fn push(&mut self, op: PointerOp) -> bool {
        match self.ops.get_mut(self.len) {
            Some(slot) => {
                *slot = op;
                self.len += 1;
                true
            }
            None => false,
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn as_slice(&self) -> &[PointerOp] {
        &self.ops[..self.len]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for PointerOps {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for PointerOps {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
