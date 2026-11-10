use serde::{Deserialize, Serialize};

// The body of the member calls that only name the member: heartbeat, unregister, and the query of poll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberRef {
    pub id: String,
}
