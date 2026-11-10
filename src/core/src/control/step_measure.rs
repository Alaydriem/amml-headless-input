use super::Step;

// The length of a step list: how many steps it holds once counted repeats are expanded, and how many ticks it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepMeasure {
    pub steps: u64,
    // None when a goal step or a repeat with no count leaves the length open
    pub ticks: Option<u64>,
}

impl StepMeasure {
    pub fn of(steps: &[Step]) -> Self {
        let mut total = Self { steps: 0, ticks: Some(0) };
        for step in steps {
            let part = Self::of_step(step);
            total.steps = total.steps.saturating_add(part.steps);
            total.ticks = match (total.ticks, part.ticks) {
                (Some(a), Some(b)) => Some(a.saturating_add(b)),
                _ => None,
            };
        }
        total
    }

    fn of_step(step: &Step) -> Self {
        let fixed = |ticks: u64| Self { steps: 1, ticks: Some(ticks) };
        match step {
            Step::Move { ticks, .. }
            | Step::Jump { ticks }
            | Step::Sneak { ticks }
            | Step::Sprint { ticks }
            | Step::Turn { ticks, .. }
            | Step::LookAt { ticks, .. }
            | Step::Wait { ticks } => fixed(u64::from(ticks.get())),
            Step::Pointer { hold_ticks, .. } => fixed(u64::from(*hold_ticks).max(1)),
            Step::Hotbar { .. } | Step::Scroll { .. } | Step::Command { .. } => fixed(1),
            Step::WalkTo { .. }
            | Step::WalkUntilDimension { .. }
            | Step::JoinServer { .. }
            | Step::LeaveWorld { .. } => Self { steps: 1, ticks: None },
            Step::Repeat { count, steps, .. } => {
                let inner = Self::of(steps);
                match count {
                    Some(count) => Self {
                        steps: inner.steps.saturating_mul(u64::from(*count)),
                        ticks: inner.ticks.map(|ticks| ticks.saturating_mul(u64::from(*count))),
                    },
                    None => Self { steps: inner.steps, ticks: None },
                }
            }
        }
    }
}
