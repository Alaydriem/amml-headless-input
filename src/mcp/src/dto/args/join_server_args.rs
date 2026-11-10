use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `join_server`: the server address and port.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct JoinServerArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    pub address: String,
    /// The server port; left out, 19132.
    pub port: Option<u16>,
    /// Duration text; left out, the goal timeout of the instance.
    pub timeout: Option<String>,
}

impl JoinServerArgs {
    pub const DEFAULT_PORT: u16 = 19132;
}
