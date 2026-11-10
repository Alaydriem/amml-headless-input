// Why the run supervisor did not take a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("a control file run is active")]
    RunActive,
    #[error("a live session is active")]
    LiveActive,
    #[error("the last run is still ending")]
    Ending,
    #[error("no run or live session is active")]
    NothingActive,
}
