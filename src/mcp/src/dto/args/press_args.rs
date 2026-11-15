use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `press`: the key to tap.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct PressArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// The key name, such as "E", "Esc", "F5" or "Enter". Case is ignored.
    pub key: String,
}
