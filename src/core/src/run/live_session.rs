use super::LiveAction;
use crate::control::{InputKey, Step, Ticks};
use crate::player::step_progress::StepProgress;
use crate::player::step_run::StepRun;
use crate::player::{InputFrame, PlayerView, PointerOp, PointerOps};

// The input of a live session. Held keys, a turn, a pointer hold and a join or leave each have a slot and run at the
// same time; a new action of one kind replaces the old one of that kind.
#[derive(Debug, Clone)]
pub(crate) struct LiveSession {
    // the ticks each key of InputKey::ALL stays held, in that order; UNTIL_END holds it until the session ends
    keys: [u32; InputKey::ALL.len()],
    rotation: Option<(Step, StepRun)>,
    pointer: Option<(Step, StepRun)>,
    travel: Option<(Step, StepRun)>,
    hotbar: Option<u8>,
    // why the last join or leave failed, until the supervisor takes it
    failure: Option<&'static str>,
    // pointer events for the next tick: scrolls and the release of a replaced hold
    pending: PointerOps,
}

impl LiveSession {
    pub const UNTIL_END: u32 = u32::MAX;

    pub fn new() -> Self {
        Self {
            keys: [0; InputKey::ALL.len()],
            rotation: None,
            pointer: None,
            travel: None,
            hotbar: None,
            failure: None,
            pending: PointerOps::new(),
        }
    }

    // `Release` gives no input; the supervisor ends the session for it
    pub fn apply(&mut self, action: LiveAction, goal_timeout: Ticks) {
        match action {
            LiveAction::Hold { keys, ticks } => {
                let left = ticks.map_or(Self::UNTIL_END, Ticks::get);
                for (slot, key) in self.keys.iter_mut().zip(InputKey::ALL) {
                    if keys.contains(key) {
                        *slot = left;
                    }
                }
            }
            LiveAction::Turn { yaw, pitch, ticks } => {
                self.rotation = Some((Step::Turn { yaw, pitch, ticks }, StepRun::new()));
            }
            LiveAction::LookAt { yaw, pitch, ticks } => {
                self.rotation = Some((Step::LookAt { yaw, pitch, ticks }, StepRun::new()));
            }
            LiveAction::Pointer { action, hold_ticks } => {
                if let Some((step, mut run)) = self.pointer.take() {
                    let mut released = InputFrame::default();
                    run.abandon(&step, &mut released);
                    for op in released.pointer.as_slice() {
                        self.pending.push(*op);
                    }
                }
                self.pointer = Some((Step::Pointer { action, hold_ticks }, StepRun::new()));
            }
            LiveAction::Hotbar { slot } => self.hotbar = Some(slot),
            LiveAction::Scroll { steps } => {
                self.pending.push(PointerOp::Scroll(steps));
            }
            LiveAction::JoinServer { host, address, port, timeout } => {
                let timeout = timeout.unwrap_or(goal_timeout);
                self.travel = Some((Step::JoinServer { host, address, port, timeout }, StepRun::new()));
            }
            LiveAction::LeaveWorld { timeout } => {
                let timeout = timeout.unwrap_or(goal_timeout);
                self.travel = Some((Step::LeaveWorld { timeout }, StepRun::new()));
            }
            LiveAction::Release => {}
        }
    }

    pub fn tick(&mut self, view: &PlayerView) -> InputFrame {
        let mut frame = InputFrame::default();
        for (left, key) in self.keys.iter_mut().zip(InputKey::ALL) {
            if *left > 0 {
                frame.keys.insert(key);
                if *left != Self::UNTIL_END {
                    *left -= 1;
                }
            }
        }
        for op in self.pending.as_slice() {
            frame.pointer.push(*op);
        }
        self.pending.clear();
        frame.hotbar = self.hotbar.take();
        Self::drive(&mut self.rotation, view, &mut frame);
        Self::drive(&mut self.pointer, view, &mut frame);
        self.drive_travel(view, &mut frame);
        frame
    }

    // one step of the join or leave alone, for a frame with no game tick; held keys, turns and the pointer wait for
    // the next tick
    pub fn advance_travel(&mut self, view: &PlayerView) -> InputFrame {
        let mut frame = InputFrame::default();
        self.drive_travel(view, &mut frame);
        frame
    }

    // the join or leave under way
    pub fn travel(&self) -> Option<&Step> {
        self.travel.as_ref().map(|(step, _)| step)
    }

    // the reason the join or leave failed on the last tick, taken once
    pub fn take_failure(&mut self) -> Option<&'static str> {
        self.failure.take()
    }

    fn drive_travel(&mut self, view: &PlayerView, frame: &mut InputFrame) {
        if let Some((step, run)) = &mut self.travel {
            match run.drive(step, view, frame) {
                StepProgress::Running => {}
                StepProgress::Failed(why) => {
                    self.failure = Some(why);
                    self.travel = None;
                }
                StepProgress::Ended | StepProgress::TimedOut => self.travel = None,
            }
        }
    }

    fn drive(slot: &mut Option<(Step, StepRun)>, view: &PlayerView, frame: &mut InputFrame) {
        if let Some((step, run)) = slot
            && run.drive(step, view, frame) != StepProgress::Running
        {
            *slot = None;
        }
    }
}
