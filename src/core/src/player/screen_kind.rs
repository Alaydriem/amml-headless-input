// The kind of screen the game shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenKind {
    InWorld,
    Title,
    Loading,
    Other(String),
}

impl ScreenKind {
    // the name `status` reports: in_world, title, loading, or the game's screen name ("unknown" when it is empty)
    pub fn name(&self) -> &str {
        match self {
            Self::InWorld => "in_world",
            Self::Title => "title",
            Self::Loading => "loading",
            Self::Other(name) if name.is_empty() => "unknown",
            Self::Other(name) => name,
        }
    }
}
