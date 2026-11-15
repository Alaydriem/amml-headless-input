use super::{JoinTarget, WorldTarget};

// What a join asks the game to open: a server by address, or a local world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinRequest {
    Server(JoinTarget),
    World(WorldTarget),
}
