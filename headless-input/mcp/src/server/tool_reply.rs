use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock};
use serde::Serialize;

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
}
