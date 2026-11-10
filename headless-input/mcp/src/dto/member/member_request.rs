use serde::{Deserialize, Serialize};

use crate::dto::LiveActionDto;

// One command for one instance. A control file travels as its text: the instance loads it with its own defaults and
// features.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum MemberRequest {
    RunControlFile(String),
    Stop,
    Status,
    Live(LiveActionDto),
}
