use serde::{Deserialize, Serialize};
use serde_json::Value;

// The result of a tool on one instance when the tool ran on every instance (`"*"`): the value, or the error text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceResult {
    pub instance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl InstanceResult {
    pub fn new(instance: String, result: Result<Value, String>) -> Self {
        match result {
            Ok(value) => Self { instance, ok: Some(value), error: None },
            Err(error) => Self { instance, ok: None, error: Some(error) },
        }
    }
}
