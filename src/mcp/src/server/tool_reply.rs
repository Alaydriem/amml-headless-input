use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock};
use serde::Serialize;
use serde_json::Value;

use crate::port::PortError;

// Turns the result of a hub call into an MCP tool result. A `PortError` becomes a tool error whose text is the
// error's `Display`.
pub struct ToolReply;

impl ToolReply {
    // `Ok` is the value as JSON text, `Err` is a tool error
    pub fn port<T: Serialize>(result: Result<T, PortError>) -> Result<CallToolResult, ErrorData> {
        match result {
            Ok(value) => Self::json(&value),
            Err(error) => Ok(CallToolResult::error(vec![ContentBlock::text(error.to_string())])),
        }
    }

    // the value as JSON text
    pub fn json<T: Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
        let text = serde_json::to_string(value).map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }

    // the image as an `image/png` block and every other field as JSON text
    pub fn screenshot(result: Result<Value, PortError>) -> Result<CallToolResult, ErrorData> {
        let mut value = match result {
            Ok(value) => value,
            Err(error) => return Self::port::<Value>(Err(error)),
        };
        let data = match value.as_object_mut().and_then(|fields| fields.remove("png_base64")) {
            Some(Value::String(data)) => data,
            _ => return Err(ErrorData::internal_error("the screenshot reply has no image", None)),
        };
        let text = serde_json::to_string(&value).map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::image(data, "image/png"), ContentBlock::text(text)]))
    }
}
