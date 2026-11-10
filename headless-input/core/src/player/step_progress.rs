// What one tick of a step left it as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepProgress {
    Running,
    // the step gave no input this tick; the next step runs in the same tick
    Ended,
    TimedOut,
    // the step cannot succeed; the text says why
    Failed(&'static str),
}
