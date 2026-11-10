use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `leave_world`.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct LeaveWorldArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// Duration text; left out, the goal timeout of the instance.
    pub timeout: Option<String>,
}
