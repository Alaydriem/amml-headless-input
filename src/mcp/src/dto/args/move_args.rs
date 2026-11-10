use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `move`: the movement keys to hold and for how long.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct MoveArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// The keys to hold: forward, back, left, right.
    pub hold: Vec<String>,
    /// Duration text ("2s" or "40t"); left out, the keys stay held until the live session ends.
    #[serde(rename = "for")]
    pub for_time: Option<String>,
}
