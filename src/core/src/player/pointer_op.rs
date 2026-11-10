use crate::control::PointerAction;

// One pointer event of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerOp {
    Press(PointerAction),
    Release(PointerAction),
    // wheel steps, positive scrolls up
    Scroll(i32),
}
