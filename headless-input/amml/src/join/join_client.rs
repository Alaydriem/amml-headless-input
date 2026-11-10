use headless_input_core::structs::JoinTarget;

// What the join service asks of the game's client. The real one calls game code (`GameJoinClient`); a test gives a
// fake. Every call runs on the game thread.
pub trait JoinClient {
    // `IMinecraftGame::isPrimaryClientInstanceReady`
    fn is_primary_ready(&self) -> bool;

    // whether the client is in a world
    fn is_in_game(&self) -> bool;

    // `ClientInstance::getTopScreenName`
    fn top_screen_name(&self) -> String;

    // `ClientInstance::requestLeaveGame(true, false)`
    fn request_leave_game(&self);

    // pushes the Ore UI route of the Play > Servers tab through the game's navigator and answers whether the route was
    // accepted. False, with no game call, when an entry is off, the navigator is null or the client is in a world
    fn open_servers_tab(&self) -> bool;

    // `ServerWorldJoiner::joinExternalServer` with the host name and port of the target: the game resolves the name
    // itself, tests the connection and joins. False when the game was not asked.
    fn join_server(&self, target: &JoinTarget) -> bool;

    // the result code the game's join callback stored since the last take or clear, taken once
    fn take_join_result(&self) -> Option<i32>;

    // drops a stored result code, so an old join's answer never counts for a new join
    fn clear_join_result(&self);
}
