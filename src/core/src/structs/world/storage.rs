// Where a world folder lives under the game data root. The order is the search order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldStorage {
    User,
    Shared,
}
