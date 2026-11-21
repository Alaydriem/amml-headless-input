use amm_core::errors::LinkError;
use amml_api::ApiError;

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
    // the loader is not ready for the input yet (before the first frame), or the game shows no HUD for a hotbar select
    #[error("{}", Self::NOT_READY)]
    NotReady,
    // the loader does not serve this input on this game build
    #[error("{}", Self::UNSUPPORTED)]
    Unsupported,
    #[error("the mod manager did not take the input: {0}")]
    Link(LinkError),
}

impl OutputError {
    const REFUSED: &'static str = "the mod manager refused the input: its panel opened or another mod took the input";
    const QUEUE_FULL: &'static str = "the mod manager pointer queue is full";
    const BUSY: &'static str = "the mod manager key queue is full";
    const NOT_READY: &'static str = "the loader is not ready for the input yet";
    const UNSUPPORTED: &'static str = "the mod manager does not support this input";
    const LINK: &'static str = "the mod manager did not take the input";

    // the error of a send the manager answered with `error`
    pub fn from_link(error: LinkError) -> Self {
        match error {
            LinkError::Api(error) => Self::from_api(error),
            other => Self::Link(other),
        }
    }

    // the error of a game call the loader answered with `error`
    pub fn from_api(error: ApiError) -> Self {
        match error {
            ApiError::Refused => Self::Refused,
            ApiError::NotReady => Self::NotReady,
            ApiError::NotAvailable => Self::Unsupported,
            other => Self::Link(LinkError::Api(other)),
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
