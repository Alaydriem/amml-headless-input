use headless_input_core::run::{RunState, RunStatus};
use serde::{Deserialize, Serialize};

use super::RunResultDto;

// The JSON form of a `RunStatus`, the reply of the `status` tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusDto {
    // idle, running or live
    pub state: String,
    pub run: Option<u64>,
    pub name: Option<String>,
    pub step: Option<usize>,
    pub ticks_in_step: Option<u32>,
    // the passes each open repeat has finished, outermost first
    pub repeat_passes: Vec<u32>,
    pub ticks: u64,
    pub last: Option<RunResultDto>,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub dimension: Option<i32>,
}

impl From<&RunStatus> for StatusDto {
    fn from(status: &RunStatus) -> Self {
        let state = match status.state {
            RunState::Idle => "idle",
            RunState::Running => "running",
            RunState::Live => "live",
        };
        Self {
            state: state.to_owned(),
            run: status.run.map(|run| run.0),
            name: status.name.as_deref().map(str::to_owned),
            step: status.cursor.map(|cursor| cursor.step),
            ticks_in_step: status.cursor.map(|cursor| cursor.ticks_in_step),
            repeat_passes: status.cursor.map(|cursor| cursor.repeat_passes().to_vec()).unwrap_or_default(),
            ticks: status.ticks,
            last: status.last.as_deref().map(RunResultDto::from),
            position: status.position,
            yaw: status.yaw,
            pitch: status.pitch,
            dimension: status.dimension,
        }
    }
}
