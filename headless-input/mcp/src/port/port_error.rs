use serde::{Deserialize, Serialize};

// Why a port call or a forwarded command failed. The MCP server returns the `Display` text as a tool error. It
// crosses the member link as JSON.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum PortError {
    #[error("the player is not in a world")]
    NotInWorld,
    #[error("the instance is busy; try again")]
    Busy,
    #[error("refused: {0}")]
    Refused(String),
    #[error("bad request: {0}")]
    Bad(String),
    #[error("timed out: {0}")]
    Timeout(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("no instance is named {0}")]
    UnknownInstance(String),
    #[error("could not resolve {host}")]
    Unresolved { host: String },
    #[error("more than one instance is connected; name one with instance (or \"*\" where allowed): [{}]", connected.join(", "))]
    NeedsInstance { connected: Vec<String> },
}
