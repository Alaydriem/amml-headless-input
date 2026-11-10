use amm_core::errors::LinkError;

// Why the mod could not take the `Input` lease. A run or live session does not start without it
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LeaseError {
    #[error("{holder} holds the input")]
    Refused { holder: String },
    #[error("the mod manager did not answer the input lease request: {0}")]
    Link(LinkError),
}
