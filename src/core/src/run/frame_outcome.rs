use super::EndReason;
use crate::player::InputFrame;

// What one game frame of the run supervisor gives the output side.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameOutcome {
    // input built on this frame, to send like a tick's frame; None leaves the input of the last tick as it is. On an
    // end it is the empty frame, so every held input is released before the end is acted on
    pub frame: Option<InputFrame>,
    pub end: Option<EndReason>,
}
