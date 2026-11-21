use std::path::Path;

use headless_input_core::structs::ScreenshotLayer;

use crate::errors::OutputError;

// what the screenshot desk needs from the loader: whether it takes screenshots, and one request. Game thread
pub trait ScreenshotPort {
    fn supports_screenshots(&self) -> bool;

    // asks for a PNG of the next frame at `path`; the sequence of the answer events
    fn request_screenshot(&self, path: &Path, layer: ScreenshotLayer) -> Result<u32, OutputError>;
}
