use crate::control::{ControlFile, Step};

// The most ticks a control file can take: its fixed length, or with goal steps and open repeats the sum of their
// timeouts. The wall-clock cap of a run is twice this.
pub(crate) struct RunLength;

impl RunLength {
    pub fn bound(file: &ControlFile) -> u64 {
        match file.total_ticks {
            Some(ticks) => u64::from(ticks),
            None => Self::of(&file.steps),
        }
    }

    fn of(steps: &[Step]) -> u64 {
        steps.iter().fold(0, |total, step| total.saturating_add(Self::of_step(step)))
    }

    fn of_step(step: &Step) -> u64 {
        match step {
            Step::Move { ticks, .. }
            | Step::Jump { ticks }
            | Step::Sneak { ticks }
            | Step::Sprint { ticks }
            | Step::Turn { ticks, .. }
            | Step::LookAt { ticks, .. }
            | Step::Wait { ticks } => u64::from(ticks.get()),
            // the press ticks, then the release tick
            Step::Pointer { hold_ticks, .. } => u64::from(*hold_ticks).max(1) + 1,
            Step::Hotbar { .. } | Step::Scroll { .. } | Step::Command { .. } => 1,
            Step::WalkTo { timeout, .. }
            | Step::WalkUntilDimension { timeout, .. }
            | Step::JoinServer { timeout, .. }
            | Step::LeaveWorld { timeout } => u64::from(timeout.get()),
            Step::Repeat { count, timeout, steps } => {
                let counted = count.map(|count| Self::of(steps).saturating_mul(u64::from(count)));
                let timed = timeout.map(|timeout| u64::from(timeout.get()));
                match (counted, timed) {
                    (Some(counted), Some(timed)) => counted.min(timed),
                    (Some(ticks), None) | (None, Some(ticks)) => ticks,
                    (None, None) => ControlFile::MAX_TICKS,
                }
            }
        }
    }
}
