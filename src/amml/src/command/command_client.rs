// What the command service asks of the game's client. The real one calls game code (`GameCommandClient`); a test gives
// a fake. Every call runs on the game thread.
pub trait CommandClient {
    // whether `ClientInstance::getLocalPlayer` gives a player
    fn has_local_player(&self) -> bool;

    // sends `text`, a slash command with its leading slash, to the server as the local player, through a
    // CommandRequestPacket. False, with nothing sent, when an entry is off or a step of the call chain is null
    fn send_command(&self, text: &str) -> bool;
}
