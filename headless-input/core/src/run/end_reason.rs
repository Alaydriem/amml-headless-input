use std::borrow::Cow;

use crate::player::PlayerView;

// Why a run or live session ended.
#[derive(Debug, Clone, PartialEq)]
pub enum EndReason {
    Done,
    // the route did not go as the file says; step is the StepCursor::step of a file run, None in a live session
    Failed { step: Option<usize>, view: PlayerView, why: Cow<'static, str> },
    Stopped,
    Aborted { why: Cow<'static, str> },
}
