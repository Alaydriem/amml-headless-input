use serde::{Deserialize, Serialize};

// The reply of `GET /member/hello`: names the program on the hub port, so a member knows it is a headless-input hub
// that speaks its protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubHello {
    pub server: String,
    pub protocol: u32,
}

impl HubHello {
    pub const SERVER: &'static str = "headless-input";
    pub const PROTOCOL: u32 = 1;

    pub fn new() -> Self {
        Self { server: Self::SERVER.to_owned(), protocol: Self::PROTOCOL }
    }

    // true when the reply comes from a hub of this protocol
    pub fn is_ours(&self) -> bool {
        self.server == Self::SERVER && self.protocol == Self::PROTOCOL
    }
}

impl Default for HubHello {
    fn default() -> Self {
        Self::new()
    }
}
