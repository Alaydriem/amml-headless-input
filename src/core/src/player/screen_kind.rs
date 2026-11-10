// The kind of screen the game shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenKind {
    InWorld,
    Title,
    Loading,
    Other(String),
}
