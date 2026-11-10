use serde::{Deserialize, Serialize};

use super::MemberRequest;

// One command the hub forwards to a member: the reply names `command`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Forwarded {
    pub command: u64,
    pub request: MemberRequest,
}
