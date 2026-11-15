use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `screenshot`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ScreenshotArgs {
    /// The instance id; may be left out when one instance is connected.
    pub instance: Option<String>,
    /// "mods" (default) includes mod overlays, "game" leaves them out.
    pub layers: Option<String>,
    /// A wider image is scaled down to this width; default 1280.
    pub max_width: Option<u32>,
}
