use serde::{Deserialize, Serialize};

use super::MemberReply;

// The body of `POST /member/reply`: the answer of member `id` to forwarded command `command`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplyPost {
    pub id: String,
    pub command: u64,
    pub reply: MemberReply,
}
