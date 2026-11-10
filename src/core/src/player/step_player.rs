use super::seek::Seek;
use super::step_level::StepLevel;
use super::step_progress::StepProgress;
use super::step_run::StepRun;
use super::{InputFrame, PlayerView, StepCursor, StepOutcome};
use crate::control::{ControlFile, Step};

// Plays a control file one tick at a time: each tick takes what the game shows and gives the full input wanted for
// that tick. Holds are measured in ticks, so a step of N ticks gives its input on exactly N calls to `tick`.
pub struct StepPlayer {
    steps: Vec<Step>,
    // levels[0] is the top list; levels[1..=depth] are the open repeats
    levels: [StepLevel; ControlFile::MAX_DEPTH + 1],
    depth: usize,
    run: StepRun,
    now: u64,
    outcome: Option<StepOutcome>,
}

impl StepPlayer {
    // the cursor moves one tick may make before it gives up until the next tick; this bounds the work of a tick on a
    // file whose steps all end at once
    pub const MAX_MOVES_PER_TICK: usize = 4 * ControlFile::MAX_STEPS as usize;

    pub fn new(file: ControlFile) -> Self {
        Self {
            steps: file.steps,
            levels: [StepLevel::default(); ControlFile::MAX_DEPTH + 1],
            depth: 0,
            run: StepRun::new(),
            now: 0,
            outcome: None,
        }
    }

    pub fn tick(&mut self, view: &PlayerView) -> InputFrame {
        let mut frame = InputFrame::default();
        if self.outcome.is_some() {
            return frame;
        }
        for _ in 0..Self::MAX_MOVES_PER_TICK {
            match self.seek(&mut frame) {
                Seek::Moved => continue,
                Seek::Done => {
                    self.outcome = Some(StepOutcome::Done);
                    break;
                }
                Seek::Leaf => {}
            }
            let Some(step) = Self::resolve(&self.steps, &self.levels[..=self.depth]) else {
                break;
            };
            match self.run.drive(step, view, &mut frame) {
                StepProgress::Running => break,
                StepProgress::Ended => self.next(),
                StepProgress::TimedOut => {
                    self.outcome = Some(StepOutcome::GoalTimedOut { step: self.ordinal(), view: view.clone() });
                    break;
                }
                StepProgress::Failed(why) => {
                    self.outcome = Some(StepOutcome::StepFailed { step: self.ordinal(), view: view.clone(), why });
                    break;
                }
            }
        }
        self.now = self.now.saturating_add(1);
        frame
    }

    pub fn current(&self) -> StepCursor {
        let mut passes = [0; ControlFile::MAX_DEPTH];
        for (slot, level) in passes.iter_mut().zip(&self.levels[1..=self.depth]) {
            *slot = level.passes;
        }
        StepCursor::new(self.ordinal(), self.run.ticks(), &passes[..self.depth])
    }

    // the step the cursor is on, or for a repeat the first step of its block; None once the run has an outcome
    pub fn current_step(&self) -> Option<&Step> {
        if self.outcome.is_some() {
            return None;
        }
        let mut step = Self::resolve(&self.steps, &self.levels[..=self.depth])?;
        for _ in 0..ControlFile::MAX_DEPTH {
            match step {
                Step::Repeat { steps, .. } => step = steps.first()?,
                _ => break,
            }
        }
        Some(step)
    }

    pub fn outcome(&self) -> Option<StepOutcome> {
        self.outcome.clone()
    }

    fn next(&mut self) {
        self.levels[self.depth].index += 1;
        self.run.reset();
    }

    // moves the cursor once toward a leaf step: closes a timed out or finished repeat, or opens one
    fn seek(&mut self, frame: &mut InputFrame) -> Seek {
        for depth in 1..=self.depth {
            let Some(Step::Repeat { timeout: Some(timeout), .. }) = Self::resolve(&self.steps, &self.levels[..depth])
            else {
                continue;
            };
            if self.now.saturating_sub(self.levels[depth].opened) >= u64::from(timeout.get()) {
                if let Some(step) = Self::resolve(&self.steps, &self.levels[..=self.depth]) {
                    self.run.abandon(step, frame);
                }
                self.depth = depth - 1;
                self.next();
                return Seek::Moved;
            }
        }
        let list = Self::list(&self.steps, &self.levels[..self.depth]);
        match list.get(self.levels[self.depth].index) {
            None if self.depth == 0 => Seek::Done,
            None => {
                let count = match Self::resolve(&self.steps, &self.levels[..self.depth]) {
                    Some(Step::Repeat { count, .. }) => *count,
                    _ => Some(0),
                };
                let level = &mut self.levels[self.depth];
                level.passes = level.passes.saturating_add(1);
                if count.is_some_and(|count| level.passes >= count) {
                    self.depth -= 1;
                    self.next();
                } else {
                    level.index = 0;
                    self.run.reset();
                }
                Seek::Moved
            }
            Some(Step::Repeat { steps, .. }) if steps.is_empty() || self.depth == ControlFile::MAX_DEPTH => {
                self.next();
                Seek::Moved
            }
            Some(Step::Repeat { .. }) => {
                self.depth += 1;
                self.levels[self.depth] = StepLevel { index: 0, passes: 0, opened: self.now };
                self.run.reset();
                Seek::Moved
            }
            Some(_) => Seek::Leaf,
        }
    }

    // the step `levels` points at; the last level's index names it within the list the earlier levels open
    fn resolve<'a>(steps: &'a [Step], levels: &[StepLevel]) -> Option<&'a Step> {
        let (last, open) = levels.split_last()?;
        Self::list(steps, open).get(last.index)
    }

    // the step list that `levels` opens: the top list, then the block of each repeat they point at
    fn list<'a>(steps: &'a [Step], levels: &[StepLevel]) -> &'a [Step] {
        let mut list = steps;
        for level in levels {
            list = match list.get(level.index) {
                Some(Step::Repeat { steps, .. }) => steps,
                _ => &[],
            };
        }
        list
    }

    // the position of the current step in the file, counting every step and repeat block in reading order
    fn ordinal(&self) -> usize {
        let mut ordinal = 0;
        let mut list: &[Step] = &self.steps;
        for (depth, level) in self.levels[..=self.depth].iter().enumerate() {
            let before = level.index.min(list.len());
            ordinal += list[..before].iter().map(Self::size).sum::<usize>();
            if depth < self.depth {
                ordinal += 1;
                list = match list.get(level.index) {
                    Some(Step::Repeat { steps, .. }) => steps,
                    _ => &[],
                };
            }
        }
        ordinal
    }

    // the steps a step holds, itself included
    fn size(step: &Step) -> usize {
        match step {
            Step::Repeat { steps, .. } => 1 + steps.iter().map(Self::size).sum::<usize>(),
            _ => 1,
        }
    }
}
