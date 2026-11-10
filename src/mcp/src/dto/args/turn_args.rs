use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `turn` (by the angles) and `look_at` (to the angles).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TurnArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// Degrees; Minecraft yaw, 0 faces +Z and 90 faces -X.
    pub yaw: f32,
    /// Degrees; positive looks down.
    #[serde(default)]
    pub pitch: f32,
    /// Duration text ("10t" or "0.5s") the turn takes.
    pub over: String,
}
