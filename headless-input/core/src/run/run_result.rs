use std::sync::Arc;

use super::{EndReason, RunId};

// How one run or live session ended.
#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    pub run: RunId,
    // the control file name; None for a live session
    pub name: Option<Arc<str>>,
    pub reason: EndReason,
    // the game ticks the run saw
    pub ticks: u64,
}
