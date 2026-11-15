use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use headless_input_core::structs::{ScreenshotLayer, ScreenshotShot};
use serde::{Deserialize, Serialize};

// The reply of `screenshot`: the PNG as base64 and the facts about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenshotDto {
    pub png_base64: String,
    // where the instance saved the PNG
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    // "mods" or "game"
    pub layers: String,
}

impl From<&ScreenshotShot> for ScreenshotDto {
    fn from(shot: &ScreenshotShot) -> Self {
        let layers = match shot.layer {
            ScreenshotLayer::Game => "game",
            ScreenshotLayer::Mods => "mods",
        };
        Self {
            png_base64: STANDARD.encode(&shot.png),
            path: shot.path.display().to_string(),
            width: shot.width,
            height: shot.height,
            source_width: shot.source_width,
            source_height: shot.source_height,
            layers: layers.to_owned(),
        }
    }
}
