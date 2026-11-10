use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `hotbar`: the slot to select.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct HotbarArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// The hotbar slot, 1 to 9.
    pub slot: i64,
}
