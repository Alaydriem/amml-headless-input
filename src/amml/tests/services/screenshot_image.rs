use std::path::Path;

use alaydriem_headless_input::services::ScreenshotImage;
use headless_input_core::structs::{ScreenshotLayer, ScreenshotRequest};
use headless_input_mcp::port::PortError;

struct Png;

impl Png {
    fn write(path: &Path, width: u32, height: u32) {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]));
        image.save(path).unwrap();
    }

    fn size(bytes: &[u8]) -> (u32, u32) {
        let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap();
        (decoded.width(), decoded.height())
    }
}

fn request() -> ScreenshotRequest {
    ScreenshotRequest { layer: ScreenshotLayer::Game, max_width: 1280 }
}

#[test]
fn wide_png_is_scaled_to_max_width() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("wide.png");
    Png::write(&path, 2560, 1440);

    let shot = ScreenshotImage::prepare(&path, &request()).unwrap();

    assert_eq!((shot.width, shot.height), (1280, 720));
    assert_eq!((shot.source_width, shot.source_height), (2560, 1440));
    assert_eq!(Png::size(&shot.png), (1280, 720));
    assert_eq!(shot.path, path);
    assert_eq!(shot.layer, ScreenshotLayer::Game);
}

#[test]
fn narrow_png_is_returned_unscaled() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("narrow.png");
    Png::write(&path, 800, 600);

    let shot = ScreenshotImage::prepare(&path, &request()).unwrap();

    assert_eq!((shot.width, shot.height), (800, 600));
    assert_eq!((shot.source_width, shot.source_height), (800, 600));
    assert_eq!(Png::size(&shot.png), (800, 600));
}

#[test]
fn missing_or_broken_file_is_an_io_error() {
    let temp = tempfile::tempdir().unwrap();
    let broken = temp.path().join("broken.png");
    std::fs::write(&broken, b"not a png").unwrap();

    let missing = ScreenshotImage::prepare(&temp.path().join("none.png"), &request());
    let bad = ScreenshotImage::prepare(&broken, &request());

    assert!(matches!(missing, Err(PortError::Io(_))), "{missing:?}");
    assert!(matches!(bad, Err(PortError::Io(_))), "{bad:?}");
}
