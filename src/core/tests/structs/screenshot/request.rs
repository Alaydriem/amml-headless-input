use headless_input_core::structs::{ScreenshotLayer, ScreenshotRequest};

#[test]
fn defaults_are_mods_and_1280() {
    let request = ScreenshotRequest::parse(None, None).unwrap();
    assert_eq!(request.layer, ScreenshotLayer::Mods);
    assert_eq!(request.max_width, 1280);
}

#[test]
fn layer_text_is_checked() {
    let game = ScreenshotRequest::parse(Some("GAME"), None).unwrap();
    assert_eq!(game.layer, ScreenshotLayer::Game);
    assert!(ScreenshotRequest::parse(Some("ui"), None).is_err());
}

#[test]
fn max_width_outside_64_to_3840_is_refused() {
    assert!(ScreenshotRequest::parse(None, Some(63)).is_err());
    assert!(ScreenshotRequest::parse(None, Some(3841)).is_err());
    assert!(ScreenshotRequest::parse(None, Some(64)).is_ok());
    assert!(ScreenshotRequest::parse(None, Some(3840)).is_ok());
}

#[test]
fn narrower_image_is_not_scaled() {
    let request = ScreenshotRequest::parse(None, None).unwrap();
    assert_eq!(request.scaled_size(800, 600), (800, 600));
}

#[test]
fn wider_image_keeps_the_aspect_ratio() {
    let request = ScreenshotRequest::parse(None, None).unwrap();
    assert_eq!(request.scaled_size(2560, 1440), (1280, 720));
    assert_eq!(request.scaled_size(2561, 1441), (1280, 720));
    assert_eq!(request.scaled_size(5000, 1).1, 1);
}
