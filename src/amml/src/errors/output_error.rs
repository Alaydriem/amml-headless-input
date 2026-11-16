use amm_abi::types::AmmStatus;
use amm_core::errors::LinkError;

// Why the manager did not take a fake key, pointer action or move. A run ends aborted on any of them, with `why` as
// its reason. None of them allocates, so the error is free on the tick path
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputError {
    // the manager panel is open, or another mod holds the input; in the second case the manager keeps the mod's held
    // keys and still takes their ups
    #[error("{}", Self::REFUSED)]
    Refused,
    #[error("{}", Self::QUEUE_FULL)]
    QueueFull,
    // the manager key queue is full
    #[error("{}", Self::BUSY)]
    Busy,
    // the manager does not know the game's keyboard controller yet, or the game has no local player in a world
    #[error("{}", Self::NOT_READY)]
    NotReady,
    // the manager is older than the call, or its game lacks the entries the call needs
    #[error("{}", Self::UNSUPPORTED)]
    Unsupported,
    #[error("the mod manager did not take the input: {0}")]
    Link(LinkError),
}

impl OutputError {
    const REFUSED: &'static str = "the mod manager refused the input: its panel opened or another mod took the input";
    const QUEUE_FULL: &'static str = "the mod manager pointer queue is full";
    const BUSY: &'static str = "the mod manager key queue is full";
    const NOT_READY: &'static str =
        "the mod manager is not ready for the input: no keyboard controller yet, or no local player in a world";
    const UNSUPPORTED: &'static str = "the mod manager does not support this input";
    const LINK: &'static str = "the mod manager did not take the input";

    // the error of a send the manager answered with `error`
    pub fn from_link(error: LinkError) -> Self {
        match error {
            LinkError::Status(code) if code == AmmStatus::NotPermitted as i32 => Self::Refused,
            LinkError::Status(code) if code == AmmStatus::QueueFull as i32 => Self::QueueFull,
            LinkError::Status(code) if code == AmmStatus::Busy as i32 => Self::Busy,
            LinkError::Status(code) if code == AmmStatus::NotReady as i32 => Self::NotReady,
            LinkError::Status(code) if code == AmmStatus::Unsupported as i32 => Self::Unsupported,
            other => Self::Link(other),
        }
    }

    // the fixed text a run ends aborted with
    pub fn why(&self) -> &'static str {
        match self {
            Self::Refused => Self::REFUSED,
            Self::QueueFull => Self::QUEUE_FULL,
            Self::Busy => Self::BUSY,
            Self::NotReady => Self::NOT_READY,
            Self::Unsupported => Self::UNSUPPORTED,
            Self::Link(_) => Self::LINK,
        }
    }
}
