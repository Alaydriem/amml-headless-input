use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `scroll`: the wheel steps.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ScrollArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// Wheel steps; positive scrolls up.
    pub steps: i32,
}
