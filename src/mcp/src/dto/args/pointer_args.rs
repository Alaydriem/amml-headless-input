use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `attack` and `use`: a click, or a hold for a duration.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct PointerArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// Duration text ("2s" or "40t") to hold the button; left out, one click.
    pub hold: Option<String>,
}
