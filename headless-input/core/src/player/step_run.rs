use std::net::Ipv4Addr;
use std::sync::Arc;

use super::join_phase::JoinPhase;
use super::step_progress::StepProgress;
use super::{InputFrame, PlayerView, PointerOp, ScreenKind, TurnPlan};
use crate::control::{InputKey, Step};
use crate::structs::JoinTarget;

// The state of the step the player is running: the ticks it has run and what it noted on its first tick.
#[derive(Debug, Clone)]
pub(crate) struct StepRun {
    ticks: u32,
    plan: Option<TurnPlan>,
    // the first known dimension of a walk_until_dimension step
    start_dimension: Option<i32>,
    join: Option<JoinPhase>,
    // a join step asked to join on a title frame
    join_asked: bool,
    // a join step saw a loading or other screen after it asked to join
    join_under_way: bool,
}

impl StepRun {
    // the ticks walk_to turns toward its target before it walks
    pub const WALK_TURN_TICKS: u32 = 5;

    pub fn new() -> Self {
        Self { ticks: 0, plan: None, start_dimension: None, join: None, join_asked: false, join_under_way: false }
    }

    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    // the step is cut short: a held pointer is released
    pub fn abandon(&mut self, step: &Step, frame: &mut InputFrame) {
        if let Step::Pointer { action, .. } = step
            && self.ticks > 0
        {
            frame.pointer.push(PointerOp::Release(*action));
        }
        self.reset();
    }

    // one tick of `step`, written into `frame`
    pub fn drive(&mut self, step: &Step, view: &PlayerView, frame: &mut InputFrame) -> StepProgress {
        let progress = match step {
            Step::Move { hold, ticks } => self.hold(hold, ticks.get(), frame),
            Step::Jump { ticks } => self.hold(&[InputKey::Jump], ticks.get(), frame),
            Step::Sneak { ticks } => self.hold(&[InputKey::Sneak], ticks.get(), frame),
            Step::Sprint { ticks } => self.hold(&[InputKey::Sprint], ticks.get(), frame),
            Step::Wait { ticks } => self.hold(&[], ticks.get(), frame),
            Step::Turn { yaw, pitch, ticks } => {
                self.rotate(|from| TurnPlan::by(from, (*yaw, *pitch), ticks.get()), ticks.get(), view, frame)
            }
            Step::LookAt { yaw, pitch, ticks } => {
                self.rotate(|from| TurnPlan::toward(from, (*yaw, *pitch), ticks.get()), ticks.get(), view, frame)
            }
            Step::Pointer { action, hold_ticks } => {
                if self.ticks >= (*hold_ticks).max(1) {
                    frame.pointer.push(PointerOp::Release(*action));
                    StepProgress::Ended
                } else {
                    if self.ticks == 0 {
                        frame.pointer.push(PointerOp::Press(*action));
                    }
                    StepProgress::Running
                }
            }
            Step::Hotbar { slot } => self.once(frame, |frame| frame.hotbar = Some(*slot)),
            Step::Scroll { steps } => self.once(frame, |frame| {
                frame.pointer.push(PointerOp::Scroll(*steps));
            }),
            Step::Command { text } => self.once(frame, |frame| frame.command = Some(Arc::clone(text))),
            Step::WalkTo { x, z, y, within, sprint, timeout } => {
                self.walk_to((*x, *y, *z), *within, *sprint, timeout.get(), view, frame)
            }
            Step::WalkUntilDimension { dimension, timeout } => {
                if self.start_dimension.is_none() {
                    self.start_dimension = view.dimension;
                }
                // only a known dimension that differs from the start ends the walk; a named walk also needs the target
                let arrived = match (view.dimension, self.start_dimension) {
                    (Some(now), Some(start)) if now != start => dimension.is_none_or(|name| now == name.id()),
                    _ => false,
                };
                if arrived {
                    StepProgress::Ended
                } else if self.ticks >= timeout.get() {
                    StepProgress::TimedOut
                } else {
                    frame.keys.insert(InputKey::Forward);
                    StepProgress::Running
                }
            }
            Step::JoinServer { host, address, port, .. } => self.join_server(host, *address, *port, view, frame),
            Step::LeaveWorld { .. } => {
                if view.screen == ScreenKind::Title {
                    StepProgress::Ended
                } else {
                    frame.leave = true;
                    StepProgress::Running
                }
            }
            Step::Repeat { .. } => StepProgress::Ended,
        };
        if progress == StepProgress::Running {
            self.ticks = self.ticks.saturating_add(1);
        }
        progress
    }

    // a join that starts in a world leaves it and waits for the title first; the step ends on a world seen after
    // another screen
    // Loading or any other screen after the join was asked counts as the join under way; the title after that is a
    // join the game refused or dropped
    fn join_server(
        &mut self,
        host: &str,
        address: Ipv4Addr,
        port: u16,
        view: &PlayerView,
        frame: &mut InputFrame,
    ) -> StepProgress {
        let joining = || JoinPhase::Joining(Arc::new(JoinTarget { host: host.to_string(), address, port }));
        let phase = self
            .join
            .get_or_insert_with(|| if view.screen == ScreenKind::InWorld { JoinPhase::Leaving } else { joining() });
        if *phase == JoinPhase::Leaving && view.screen == ScreenKind::Title {
            *phase = joining();
        }
        match phase {
            JoinPhase::Leaving => frame.leave = true,
            JoinPhase::Joining(_) if view.screen == ScreenKind::InWorld => return StepProgress::Ended,
            JoinPhase::Joining(target) => {
                match view.screen {
                    ScreenKind::Title if self.join_under_way => {
                        return StepProgress::Failed("the join failed: the game returned to the menus");
                    }
                    ScreenKind::Title => self.join_asked = true,
                    ScreenKind::Loading | ScreenKind::Other(_) if self.join_asked => self.join_under_way = true,
                    _ => {}
                }
                frame.join = Some(Arc::clone(target));
            }
        }
        StepProgress::Running
    }

    fn hold(&self, keys: &[InputKey], ticks: u32, frame: &mut InputFrame) -> StepProgress {
        if self.ticks >= ticks {
            return StepProgress::Ended;
        }
        for key in keys {
            frame.keys.insert(*key);
        }
        StepProgress::Running
    }

    fn once(&self, frame: &mut InputFrame, write: impl FnOnce(&mut InputFrame)) -> StepProgress {
        if self.ticks >= 1 {
            return StepProgress::Ended;
        }
        write(frame);
        StepProgress::Running
    }

    fn rotate(
        &mut self,
        make: impl FnOnce((f32, f32)) -> TurnPlan,
        ticks: u32,
        view: &PlayerView,
        frame: &mut InputFrame,
    ) -> StepProgress {
        if self.ticks >= ticks {
            return StepProgress::Ended;
        }
        let plan = *self.plan.get_or_insert_with(|| make((view.yaw, view.pitch)));
        frame.rotation = Some(plan.at(self.ticks));
        StepProgress::Running
    }

    fn walk_to(
        &mut self,
        target: (f32, Option<f32>, f32),
        within: f32,
        sprint: bool,
        timeout: u32,
        view: &PlayerView,
        frame: &mut InputFrame,
    ) -> StepProgress {
        let turning = self.ticks < Self::WALK_TURN_TICKS;
        if (!turning || self.ticks == 0) && Self::distance(view.position, target) <= within {
            return StepProgress::Ended;
        }
        if self.ticks >= timeout {
            return StepProgress::TimedOut;
        }
        let bearing = TurnPlan::bearing(view.position, target.0, target.2);
        let plan = *self.plan.get_or_insert_with(|| {
            TurnPlan::toward((view.yaw, view.pitch), (bearing, view.pitch), Self::WALK_TURN_TICKS)
        });
        if turning {
            frame.rotation = Some(plan.at(self.ticks));
        } else {
            frame.keys.insert(InputKey::Forward);
            if sprint {
                frame.keys.insert(InputKey::Sprint);
            }
            frame.rotation = Some((bearing, plan.target().1));
        }
        StepProgress::Running
    }

    // horizontal distance, or full distance when the target names a height
    fn distance(position: [f32; 3], target: (f32, Option<f32>, f32)) -> f32 {
        let dx = target.0 - position[0];
        let dz = target.2 - position[2];
        let dy = target.1.map_or(0.0, |y| y - position[1]);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
