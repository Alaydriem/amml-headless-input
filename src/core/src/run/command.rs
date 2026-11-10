use super::LiveAction;
use crate::control::ControlFile;

// A request for the run supervisor, queued by the MCP side and applied on the game thread.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    RunFile { file: ControlFile },
    Live(LiveAction),
    Stop,
}
