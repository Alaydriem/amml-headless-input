use headless_input_core::run::{EndReason, RunResult};
use serde::{Deserialize, Serialize};

// The JSON form of a `RunResult`: how one run or live session ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunResultDto {
    pub run: u64,
    pub name: Option<String>,
    // done, failed, stopped or aborted
    pub reason: String,
    pub why: Option<String>,
    // the step a failed file run was on
    pub step: Option<usize>,
    pub ticks: u64,
}

impl From<&RunResult> for RunResultDto {
    fn from(result: &RunResult) -> Self {
        let (reason, why, step) = match &result.reason {
            EndReason::Done => ("done", None, None),
            EndReason::Failed { step, why, .. } => ("failed", Some(why.to_string()), *step),
            EndReason::Stopped => ("stopped", None, None),
            EndReason::Aborted { why } => ("aborted", Some(why.to_string()), None),
        };
        Self {
            run: result.run.0,
            name: result.name.as_deref().map(str::to_owned),
            reason: reason.to_owned(),
            why,
            step,
            ticks: result.ticks,
        }
    }
}
