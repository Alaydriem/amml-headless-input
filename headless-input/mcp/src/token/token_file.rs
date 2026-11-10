use serde::{Deserialize, Serialize};

use super::IssuedToken;

/// The JSON form of `mcp.tokens.json`: `{"tokens":[...]}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenFile {
    #[serde(default)]
    pub tokens: Vec<IssuedToken>,
}
