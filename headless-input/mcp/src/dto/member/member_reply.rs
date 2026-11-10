use serde::{Deserialize, Serialize};

use crate::dto::StatusDto;
use crate::port::PortError;

// What an instance answers to one `MemberRequest`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum MemberReply {
    Run(u64),
    Done,
    Status(Box<StatusDto>),
    Failed(PortError),
}
