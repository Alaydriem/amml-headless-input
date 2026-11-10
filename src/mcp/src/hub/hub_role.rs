// What a hub node is now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubRole {
    // holds the hub port and serves MCP for every instance
    Hub,
    // registered with the hub
    Member,
    // no MCP: not yet registered, stopped, or the program on the port is not a headless-input hub
    Off,
    // registered, but the hub has not answered a heartbeat for `MemberLink::STALL_AFTER`
    Stalled,
}

impl HubRole {
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Hub => 0,
            Self::Member => 1,
            Self::Off => 2,
            Self::Stalled => 3,
        }
    }

    pub(crate) fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Hub,
            1 => Self::Member,
            3 => Self::Stalled,
            _ => Self::Off,
        }
    }
}
