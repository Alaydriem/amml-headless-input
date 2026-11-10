// One structural move of the player's cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Seek {
    // the cursor is on a step that gives input
    Leaf,
    Moved,
    Done,
}
