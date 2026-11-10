use std::net::{Ipv4Addr, TcpListener};

// A loopback port that was free a moment ago.
pub struct FreePort;

impl FreePort {
    pub fn pick() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap().local_addr().unwrap().port()
    }
}
