use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `join_world`: the world name or folder id.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct JoinWorldArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// The world name, or its folder id.
    pub world: String,
    /// Duration text; left out, the goal timeout of the instance.
    pub timeout: Option<String>,
}
