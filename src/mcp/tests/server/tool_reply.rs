//! The screenshot tool result: an image block and a text block without the image data.

use headless_input_mcp::PortError;
use headless_input_mcp::server::ToolReply;
use serde_json::{Value, json};

struct Body;

impl Body {
    fn value() -> Value {
        json!({
            "png_base64": "AQID",
            "path": "shots/a.png",
            "width": 64,
            "height": 36,
            "source_width": 128,
            "source_height": 72,
            "layers": "mods",
        })
    }
}

#[test]
fn screenshot_result_has_an_image_and_a_text_block() {
    let result = ToolReply::screenshot(Ok(Body::value())).unwrap();
    assert_ne!(result.is_error, Some(true));
    let images: Vec<_> = result.content.iter().filter_map(|block| block.as_image()).collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].mime_type, "image/png");
    assert_eq!(images[0].data, "AQID");
    let texts: Vec<_> = result.content.iter().filter_map(|block| block.as_text()).collect();
    assert_eq!(texts.len(), 1);
    let text: Value = serde_json::from_str(&texts[0].text).unwrap();
    assert!(text.get("png_base64").is_none(), "{text}");
    assert_eq!(text["path"], "shots/a.png");
    assert_eq!(text["width"], 64);
}

#[test]
fn screenshot_error_is_a_tool_error() {
    let result = ToolReply::screenshot(Err(PortError::Refused("no".to_owned()))).unwrap();
    assert_eq!(result.is_error, Some(true));
    assert!(result.content.iter().all(|block| block.as_image().is_none()));
}
