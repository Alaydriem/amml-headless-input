use std::net::TcpListener;

// Why a member link ended.
#[derive(Debug)]
pub enum LinkEnd {
    // the hub went away (or never answered): try the bind, else register again
    Lost,
    // the hub did not take the registration; try again later
    Refused,
    // another game registered this instance id (409): register again under a distinct id
    Taken,
    // the program on the port gave a complete HTTP reply that is not a headless-input hub
    Foreign,
    // the program on the port broke the connection or did not speak HTTP; a hub that shuts down does this too
    Garbled,
    // the node stops
    Stopped,
    // the hub stalled and its port came free: this node holds it now
    TakeOver(TcpListener),
}
