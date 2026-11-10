use serde::{Deserialize, Serialize};

// The reply of `run_control_file` on one instance: the id of the run it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStarted {
    pub run: u64,
}
