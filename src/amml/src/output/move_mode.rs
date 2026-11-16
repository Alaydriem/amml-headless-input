// How the movement keys reach the manager: through the move command, or as fake keys from a manager without it.
// `Unknown` until the first movement of a run
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MoveMode {
    #[default]
    Unknown,
    Command,
    Keys,
}
