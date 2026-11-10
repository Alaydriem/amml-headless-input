use headless_input_core::run::RunId;
use headless_input_mcp::port::PortError;

// where the answer to one queued command stands
#[derive(Debug, Clone, PartialEq)]
pub enum ReplyState {
    Waiting,
    Answered(Result<RunId, PortError>),
    // the asker stopped waiting; the game thread drops the command unrun
    Abandoned,
}
