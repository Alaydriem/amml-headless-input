use super::PlayerView;

// How a run of the player ended.
#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    Done,
    // a goal step passed its timeout; step is its StepCursor::step, view what the game showed on that tick
    GoalTimedOut { step: usize, view: PlayerView },
    // a step cannot succeed; `why` names the cause
    StepFailed { step: usize, view: PlayerView, why: &'static str },
}
