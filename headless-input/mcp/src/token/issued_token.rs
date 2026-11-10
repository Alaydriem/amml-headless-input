use serde::{Deserialize, Serialize};

/// One token: the token, the client that asked for it, when (ms since the Unix epoch), and whether it is the member
/// token the instances present to the hub (a pairing token otherwise).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuedToken {
    pub token: String,
    pub client: String,
    pub issued_ms: u64,
    #[serde(default)]
    pub member: bool,
}
