use serde::{Deserialize, Serialize};

/// The body of `POST /pair`: the name of the client that asks for a token. The hub game asks the person to allow it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairRequest {
    pub client: String,
}

impl PairRequest {
    /// The longest client name, in characters.
    pub const MAX_CLIENT: usize = 64;

    /// True when the name has 1 to 64 characters and no control characters.
    pub fn is_valid(&self) -> bool {
        let count = self.client.chars().count();
        (1..=Self::MAX_CLIENT).contains(&count) && !self.client.chars().any(char::is_control)
    }
}
