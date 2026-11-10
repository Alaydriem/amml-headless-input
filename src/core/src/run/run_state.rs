// What the run supervisor is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Idle,
    // a control file plays
    Running,
    // live calls drive the player
    Live,
}
