// Why a command step could not send its command. A run ends aborted on either. Neither allocates, so the error is free
// on the frame path
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    // the client has no local player: it is not in a world
    #[error("{}", Self::NO_PLAYER)]
    NoPlayer,
    // an entry is off, or a step of the game call chain was null
    #[error("{}", Self::NOT_SENT)]
    NotSent,
}

impl CommandError {
    const NO_PLAYER: &'static str = "the command could not be sent: there is no local player";
    const NOT_SENT: &'static str = "the command could not be sent: the game did not take it";

    // the fixed text a run ends aborted with
    pub fn why(&self) -> &'static str {
        match self {
            Self::NoPlayer => Self::NO_PLAYER,
            Self::NotSent => Self::NOT_SENT,
        }
    }
}
