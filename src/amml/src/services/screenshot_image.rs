use std::io::Cursor;
use std::path::Path;

use headless_input_core::structs::{ScreenshotRequest, ScreenshotShot};
use headless_input_mcp::port::PortError;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};

// reads the PNG the manager wrote and scales it down to the requested width. Port thread: it decodes and encodes
pub struct ScreenshotImage;

impl ScreenshotImage {
    pub fn prepare(path: &Path, request: &ScreenshotRequest) -> Result<ScreenshotShot, PortError> {
        let source = image::open(path).map_err(|error| PortError::Io(error.to_string()))?;
        let (source_width, source_height) = (source.width(), source.height());
        let (width, height) = request.scaled_size(source_width, source_height);
        let scaled = if (width, height) == (source_width, source_height) {
            source
        } else {
            DynamicImage::ImageRgba8(image::imageops::resize(&source, width, height, FilterType::Triangle))
        };
        let mut png = Vec::new();
        scaled
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .map_err(|error| PortError::Io(error.to_string()))?;
        Ok(ScreenshotShot {
            png,
            path: path.to_path_buf(),
            width,
            height,
            source_width,
            source_height,
            layer: request.layer,
        })
    }
}
