use amm_abi::types::AmmStatus;
use amm_core::errors::LinkError;

// Why the manager did not take a fake key or pointer action. A run ends aborted on any of them. None of them
// allocates, so the error is free on the tick path
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputError {
    // the manager panel is open, or another mod holds the input; in the second case the manager keeps the mod's held
    // keys and still takes their ups
    #[error("{}", Self::REFUSED)]
    Refused,
    #[error("{}", Self::QUEUE_FULL)]
    QueueFull,
    #[error("the mod manager did not take the input: {0}")]
    Link(LinkError),
}

impl OutputError {
    const REFUSED: &'static str = "the mod manager refused the input: its panel opened or another mod took the input";
    const QUEUE_FULL: &'static str = "the mod manager pointer queue is full";
    const LINK: &'static str = "the mod manager did not take the input";

    // the error of a send the manager answered with `error`
    pub fn from_link(error: LinkError) -> Self {
        match error {
            LinkError::Status(code) if code == AmmStatus::NotPermitted as i32 => Self::Refused,
            LinkError::Status(code) if code == AmmStatus::QueueFull as i32 => Self::QueueFull,
            other => Self::Link(other),
        }
    }

    // the fixed text a run ends aborted with
    pub fn why(&self) -> &'static str {
        match self {
            Self::Refused => Self::REFUSED,
            Self::QueueFull => Self::QUEUE_FULL,
            Self::Link(_) => Self::LINK,
        }
    }
}
