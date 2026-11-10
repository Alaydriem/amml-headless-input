use serde::{Deserialize, Serialize};

/// The `200` body of `POST /pair`: the new bearer token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairReply {
    pub token: String,
}
