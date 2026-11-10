use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::join_result_codes::JoinResultCodes;
use super::live_session::LiveSession;
use super::run_length::RunLength;
use super::travel::Travel;
use super::{Command, EndReason, FrameOutcome, LiveAction, RunDefaults, RunId, RunResult, RunState, RunStatus};
use crate::control::{ControlFile, Step, Ticks};
use crate::errors::Refusal;
use crate::player::{InputFrame, PlayerView, ScreenKind, StepOutcome, StepPlayer};
use crate::traits::Clock;

// Owns the one active run or live session. `on_tick` gives the input of each game tick. `on_frame` applies the
// wall-clock safety limits and reports an end once. The game gives no tick at the title or while it loads, so on a
// frame with no tick since the last one, outside a world, `on_frame` advances a join or leave step itself. A loading
// frame during a portal walk is travel too: it restarts the watchdog and skips the world-left rule. Every end
// empties the input first: the frame `on_frame` gives with an end is empty and every later tick gives an empty frame,
// so the output side releases all held input before it releases the lease.
pub struct RunSupervisor<C: Clock> {
    defaults: RunDefaults,
    clock: C,
    state: RunState,
    run: RunId,
    name: Option<Arc<str>>,
    player: Option<StepPlayer>,
    live: LiveSession,
    started: Instant,
    last_tick: Instant,
    last_call: Instant,
    wall_cap: Duration,
    ticks: u64,
    travel: Option<Travel>,
    // a game tick arrived since the last frame; a frame then does not advance the run
    ticked: bool,
    // whether the last frame of the active run was in a world; None before its first frame
    was_in_world: Option<bool>,
    // the end not yet reported by on_frame
    ended: Option<EndReason>,
    // the end not yet taken by take_end
    result: Option<RunResult>,
    last: Option<Arc<RunResult>>,
}

impl<C: Clock> RunSupervisor<C> {
    // the shortest wall-clock cap, so a run of a few ticks survives one slow frame
    pub const MIN_WALL_CAP: Duration = Duration::from_secs(1);

    pub fn new(defaults: RunDefaults, clock: C) -> Self {
        let now = clock.now();
        Self {
            defaults,
            clock,
            state: RunState::Idle,
            run: RunId(0),
            name: None,
            player: None,
            live: LiveSession::new(),
            started: now,
            last_tick: now,
            last_call: now,
            wall_cap: Self::MIN_WALL_CAP,
            ticks: 0,
            travel: None,
            ticked: false,
            was_in_world: None,
            ended: None,
            result: None,
            last: None,
        }
    }

    // takes new settings; an active run keeps its wall-clock cap
    pub fn set_defaults(&mut self, defaults: RunDefaults) {
        self.defaults = defaults;
    }

    pub fn on_tick(&mut self, view: &PlayerView) -> InputFrame {
        let now = self.clock.now();
        self.last_tick = now;
        self.ticked = true;
        match self.state {
            RunState::Idle => InputFrame::default(),
            RunState::Running => {
                self.ticks = self.ticks.saturating_add(1);
                self.advance_file(view, now)
            }
            RunState::Live => {
                self.ticks = self.ticks.saturating_add(1);
                let frame = self.live.tick(view);
                self.after_live_travel(view, frame)
            }
        }
    }

    pub fn on_frame(&mut self, view: &PlayerView, panel_open: bool, in_world: bool) -> FrameOutcome {
        let ticked = std::mem::replace(&mut self.ticked, false);
        let mut frame = None;
        if self.state != RunState::Idle {
            let now = self.clock.now();
            if !ticked && !in_world && self.travel.is_some() {
                frame = Some(self.advance_travel(view, now));
            }
            if self.state != RunState::Idle {
                let portal_load = self.portal_load(view);
                if portal_load {
                    self.last_tick = now;
                }
                if let Some(reason) = self.check(view, panel_open, in_world, portal_load, now) {
                    self.finish(reason);
                } else if !portal_load {
                    self.was_in_world = Some(in_world);
                }
            }
        }
        let end = self.ended.take();
        if end.is_some() {
            frame = Some(InputFrame::default());
        }
        FrameOutcome { frame, end }
    }

    pub fn apply(&mut self, command: Command) -> Result<RunId, Refusal> {
        let now = self.clock.now();
        match command {
            Command::RunFile { file } => match self.state {
                RunState::Running => Err(Refusal::RunActive),
                RunState::Live => Err(Refusal::LiveActive),
                RunState::Idle if self.ended.is_some() => Err(Refusal::Ending),
                RunState::Idle => Ok(self.start_file(file, now)),
            },
            Command::Live(action) => match self.state {
                RunState::Running => Err(Refusal::RunActive),
                RunState::Idle if self.ended.is_some() => Err(Refusal::Ending),
                RunState::Idle if action == LiveAction::Release => Err(Refusal::NothingActive),
                RunState::Idle => {
                    self.begin(RunState::Live, None, now);
                    self.live_call(action, now);
                    Ok(self.run)
                }
                RunState::Live => {
                    let run = self.run;
                    if action == LiveAction::Release {
                        self.finish(EndReason::Done);
                    } else {
                        self.live_call(action, now);
                    }
                    Ok(run)
                }
            },
            Command::Stop => match self.state {
                RunState::Idle => Err(Refusal::NothingActive),
                RunState::Running | RunState::Live => {
                    let run = self.run;
                    self.finish(EndReason::Stopped);
                    Ok(run)
                }
            },
        }
    }

    // ends the active run as aborted, for a cause outside the supervisor: the mod is disabled, the output was refused
    pub fn abort(&mut self, why: &'static str) {
        if self.state != RunState::Idle {
            self.finish(EndReason::Aborted { why: Cow::Borrowed(why) });
        }
    }

    // the game answered the join it was asked with `code`. A code that means the join failed ends the run or live
    // session failed at once, with the code in the reason; true when it did. Any other code, or a run with no join
    // step under way, changes nothing
    pub fn join_refused(&mut self, view: &PlayerView, code: i32) -> bool {
        let Some(why) = JoinResultCodes::failure(code) else {
            return false;
        };
        let joining = match self.state {
            RunState::Running => {
                matches!(self.player.as_ref().and_then(StepPlayer::current_step), Some(Step::JoinServer { .. }))
            }
            RunState::Live => matches!(self.live.travel(), Some(Step::JoinServer { .. })),
            RunState::Idle => false,
        };
        if !joining {
            return false;
        }
        self.finish(EndReason::Failed { step: self.step(), view: view.clone(), why });
        true
    }

    // whether a run or live session is active
    pub fn is_active(&self) -> bool {
        self.state != RunState::Idle
    }

    pub fn status(&self, view: &PlayerView) -> RunStatus {
        let active = self.state != RunState::Idle;
        RunStatus {
            state: self.state,
            run: active.then_some(self.run),
            name: self.name.clone(),
            cursor: self.player.as_ref().map(StepPlayer::current),
            ticks: if active { self.ticks } else { 0 },
            last: self.last.clone(),
            position: view.position,
            yaw: view.yaw,
            pitch: view.pitch,
            dimension: view.dimension,
        }
    }

    pub fn take_end(&mut self) -> Option<RunResult> {
        self.result.take()
    }

    fn start_file(&mut self, file: ControlFile, now: Instant) -> RunId {
        let bound = RunLength::bound(&file);
        let cap = Duration::from_secs_f64(bound.saturating_mul(2) as f64 / Ticks::PER_SECOND);
        self.wall_cap = cap.max(Self::MIN_WALL_CAP);
        let name = Some(Arc::from(file.name.as_str()));
        self.begin(RunState::Running, name, now);
        let player = StepPlayer::new(file);
        self.travel = player.current_step().and_then(|step| Travel::new(step, now, Some(player.current())));
        self.player = Some(player);
        self.run
    }

    fn begin(&mut self, state: RunState, name: Option<Arc<str>>, now: Instant) {
        self.state = state;
        self.run = RunId(self.run.0.saturating_add(1));
        self.name = name;
        self.started = now;
        self.last_tick = now;
        self.last_call = now;
        self.ticks = 0;
        self.travel = None;
        self.ticked = false;
        self.was_in_world = None;
        self.player = None;
        self.live = LiveSession::new();
    }

    fn live_call(&mut self, action: LiveAction, now: Instant) {
        self.last_call = now;
        let travels = matches!(action, LiveAction::JoinServer { .. } | LiveAction::LeaveWorld { .. });
        self.live.apply(action, self.defaults.goal_timeout);
        if travels {
            self.travel = self.live.travel().and_then(|step| Travel::new(step, now, None));
        }
    }

    // a frame with no game tick: one step of the join or leave under way
    fn advance_travel(&mut self, view: &PlayerView, now: Instant) -> InputFrame {
        match self.state {
            RunState::Running => self.advance_file(view, now),
            RunState::Live => {
                let frame = self.live.advance_travel(view);
                self.after_live_travel(view, frame)
            }
            RunState::Idle => InputFrame::default(),
        }
    }

    // clears the travel when the join or leave ended; a failed one ends the session, with the empty frame
    fn after_live_travel(&mut self, view: &PlayerView, frame: InputFrame) -> InputFrame {
        if let Some(why) = self.live.take_failure() {
            self.finish(EndReason::Failed { step: None, view: view.clone(), why: Cow::Borrowed(why) });
            return InputFrame::default();
        }
        if self.live.travel().is_none() && self.travel.take().is_some() {
            self.last_call = self.clock.now();
        }
        frame
    }

    fn advance_file(&mut self, view: &PlayerView, now: Instant) -> InputFrame {
        let Some(player) = self.player.as_mut() else {
            return InputFrame::default();
        };
        let frame = player.tick(view);
        if let Some(outcome) = player.outcome() {
            let reason = match outcome {
                StepOutcome::Done => EndReason::Done,
                StepOutcome::GoalTimedOut { step, view } => {
                    EndReason::Failed { step: Some(step), view, why: Cow::Borrowed("the goal step passed its timeout") }
                }
                StepOutcome::StepFailed { step, view, why } => {
                    EndReason::Failed { step: Some(step), view, why: Cow::Borrowed(why) }
                }
            };
            self.finish(reason);
            return InputFrame::default();
        }
        let cursor = player.current();
        self.travel = match (player.current_step(), self.travel) {
            (Some(_), Some(travel)) if travel.same_step(&cursor) => Some(travel),
            (Some(step), _) => Travel::new(step, now, Some(cursor)),
            (None, _) => None,
        };
        frame
    }

    // a loading screen while the current file step is a portal walk: the dimension change, not a left world
    fn portal_load(&self, view: &PlayerView) -> bool {
        self.state == RunState::Running
            && view.screen == ScreenKind::Loading
            && matches!(self.player.as_ref().and_then(StepPlayer::current_step), Some(Step::WalkUntilDimension { .. }))
    }

    fn check(
        &self,
        view: &PlayerView,
        panel_open: bool,
        in_world: bool,
        portal_load: bool,
        now: Instant,
    ) -> Option<EndReason> {
        let paused = self.travel.is_some() || portal_load;
        if !paused && self.was_in_world == Some(true) && !in_world {
            return Some(EndReason::Aborted { why: Cow::Borrowed("the world was left") });
        }
        if panel_open {
            return Some(EndReason::Stopped);
        }
        match &self.travel {
            Some(travel) if travel.expired(now) => {
                return Some(EndReason::Failed { step: self.step(), view: view.clone(), why: travel.why().into() });
            }
            Some(_) => {}
            None if portal_load => {}
            None => {
                if in_world && let ScreenKind::Other(name) = &view.screen {
                    let why = format!("the screen {name} opened over the world");
                    return Some(EndReason::Failed { step: self.step(), view: view.clone(), why: Cow::Owned(why) });
                }
                if now.saturating_duration_since(self.last_tick) >= self.defaults.watchdog.duration() {
                    return Some(EndReason::Aborted { why: Cow::Borrowed("no game tick for the watchdog time") });
                }
            }
        }
        if self.state == RunState::Running && now.saturating_duration_since(self.started) >= self.wall_cap {
            return Some(EndReason::Aborted { why: Cow::Borrowed("the run passed twice its length") });
        }
        if self.state == RunState::Live
            && self.travel.is_none()
            && now.saturating_duration_since(self.last_call) >= self.defaults.live_idle.duration()
        {
            return Some(EndReason::Aborted { why: Cow::Borrowed("no live call for the idle time") });
        }
        None
    }

    fn step(&self) -> Option<usize> {
        self.player.as_ref().map(|player| player.current().step)
    }

    fn finish(&mut self, reason: EndReason) {
        let result = RunResult { run: self.run, name: self.name.take(), reason: reason.clone(), ticks: self.ticks };
        self.last = Some(Arc::new(result.clone()));
        self.result = Some(result);
        self.ended = Some(reason);
        self.state = RunState::Idle;
        self.player = None;
        self.live = LiveSession::new();
        self.travel = None;
        self.was_in_world = None;
    }
}
