use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use socket2::{Domain, Protocol, Socket, Type};

// Binds the hub port on `127.0.0.1` so that no second socket can share it: the bind of every instance races, and the
// OS gives the port to exactly one.
pub struct ExclusiveListener;

impl ExclusiveListener {
    pub const BACKLOG: i32 = 128;

    // a non-blocking listener on `127.0.0.1:<port>`; `AddrInUse` (or `PermissionDenied` on Windows) when another
    // socket holds the port
    pub fn bind(port: u16) -> io::Result<TcpListener> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        // on Windows SO_REUSEADDR would let a second socket bind the port too; elsewhere it only allows a bind over
        // connections in TIME_WAIT, and two listeners still cannot share the port
        socket.set_reuse_address(!cfg!(windows))?;
        socket.bind(&SocketAddr::from((Ipv4Addr::LOCALHOST, port)).into())?;
        socket.listen(Self::BACKLOG)?;
        socket.set_nonblocking(true)?;
        Ok(socket.into())
    }
}
