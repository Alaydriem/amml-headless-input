use std::sync::Arc;

use super::{RunId, RunResult, RunState};
use crate::player::StepCursor;

// What `status` reports: the active run, the last result and where the player is. Cloning it copies no text.
#[derive(Debug, Clone, PartialEq)]
pub struct RunStatus {
    pub state: RunState,
    pub run: Option<RunId>,
    // the control file name of the active run
    pub name: Option<Arc<str>>,
    // the step of the active file run
    pub cursor: Option<StepCursor>,
    // the game ticks the active run has seen
    pub ticks: u64,
    pub last: Option<Arc<RunResult>>,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    // None while the dimension is not known
    pub dimension: Option<i32>,
}
