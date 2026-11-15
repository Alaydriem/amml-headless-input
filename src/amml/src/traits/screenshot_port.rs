use std::path::Path;

use headless_input_core::structs::ScreenshotLayer;

use crate::errors::OutputError;

// what the screenshot desk needs from the mod manager: whether it takes screenshots, and one request. Game thread
pub trait ScreenshotPort {
    fn supports_screenshots(&self) -> bool;

    // takes the `Overlay` lease the manager asks for before a screenshot; false when it is not granted
    fn acquire_overlay(&self) -> bool;

    fn release_overlay(&self);

    // asks for a PNG of the next Present at `path`; the manager's sequence for the answer events
    fn request_screenshot(&self, path: &Path, layer: ScreenshotLayer) -> Result<u32, OutputError>;
}
