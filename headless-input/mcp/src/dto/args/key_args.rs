use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `jump`, `sneak` and `sprint`: for how long to hold the key.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct KeyArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// Duration text ("2s" or "40t"); left out, the key stays held until the live session ends.
    #[serde(rename = "for")]
    pub for_time: Option<String>,
}
