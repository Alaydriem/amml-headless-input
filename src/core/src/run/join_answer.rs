// The result code the game gave a join it took. A server join and a world start use different codes, so the kind of
// join travels with the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinAnswer {
    // the code of a server join's result callback
    Server(i32),
    // the code the local world start returned
    World(i32),
}
