use std::net::Ipv4Addr;

// The server a join step connects to. `host` is what the user gave, which the game shows as the server name;
// `address` is the IPv4 address it resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinTarget {
    pub host: String,
    pub address: Ipv4Addr,
    pub port: u16,
}
