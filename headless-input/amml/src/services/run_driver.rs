use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use headless_input_core::player::{InputFrame, PlayerView, ScreenKind};
use headless_input_core::run::{
    Command, CommandQueue, EndReason, LiveAction, RunDefaults, RunId, RunResult, RunStatus, RunSupervisor,
};
use headless_input_core::structs::{JoinTarget, SystemClock};
use headless_input_mcp::port::PortError;

use crate::command::{CommandClient, CommandService};
use crate::join::{JoinClient, JoinService};
use crate::output::{InputOutput, LeaseHolder};
use crate::services::{CommandInbox, CommandReply};
use crate::traits::ManagerPort;
use crate::types::FrameView;

// The run supervisor and what carries its decisions to the game, on the game thread: each tick's input goes to the
// manager; each frame gives the supervisor its frame, sends the input it built, ends a run (held input released, then
// the lease), applies the queued commands, drives the join service and writes the status snapshot. A run or live
// session starts only after the `Input` lease is taken. The command buffers are reused, so a frame with no command
// does not allocate
pub struct RunDriver {
    supervisor: RunSupervisor<SystemClock>,
    output: InputOutput,
    lease: LeaseHolder,
    join: JoinService,
    command: CommandService,
    commands: Vec<Command>,
    replies: Vec<Arc<CommandReply>>,
    // the join and leave of the last input sent; each frame asks the join service for them again
    wanted_join: Option<Arc<JoinTarget>>,
    wanted_leave: bool,
}

impl RunDriver {
    pub const DISABLED: &'static str = "the mod was disabled";
    pub const INPUT_TAKEN: &'static str = "another mod took the input";
    pub const PANICKED: &'static str = "the mod met an internal error";
    pub const NO_MANAGER: &'static str = "the mod manager is not attached, so the input cannot be blocked";

    pub fn new(defaults: RunDefaults) -> Self {
        Self {
            supervisor: RunSupervisor::new(defaults, SystemClock::new()),
            output: InputOutput::new(),
            lease: LeaseHolder::new(),
            join: JoinService::new(),
            command: CommandService::new(),
            commands: Vec::with_capacity(CommandQueue::CAPACITY),
            replies: Vec::with_capacity(CommandQueue::CAPACITY),
            wanted_join: None,
            wanted_leave: false,
        }
    }

    pub fn set_defaults(&mut self, defaults: RunDefaults) {
        self.supervisor.set_defaults(defaults);
    }

    // the manager let go of the mod's fake input itself
    pub fn forget_output(&mut self) {
        self.output.forget();
    }

    pub fn status(&self, view: &PlayerView) -> RunStatus {
        self.supervisor.status(view)
    }

    // one game tick of the local player
    pub fn tick<P: ManagerPort>(&mut self, view: &PlayerView, port: Option<&P>) {
        let frame = self.supervisor.on_tick(view);
        self.send(&frame, port, view);
    }

    // one game frame. `in_world` is true whenever the screen is the world
    pub fn frame<P: ManagerPort, J: JoinClient, C: CommandClient>(
        &mut self,
        frame: &FrameView,
        port: Option<&P>,
        join: &J,
        command: &C,
        inbox: &CommandInbox,
        status: &Mutex<RunStatus>,
    ) {
        let view = &frame.view;
        let in_world = frame.in_world || view.screen == ScreenKind::InWorld;
        let panel_open = port.is_some_and(|port| port.panel_open());
        let outcome = self.supervisor.on_frame(view, panel_open, in_world);
        if let Some(input) = &outcome.frame {
            self.send(input, port, view);
        }
        if outcome.end.is_some() {
            self.finish(port);
        }
        self.take_commands(inbox, port);
        if let Some(target) = &self.wanted_join {
            self.join.request(Arc::clone(target));
        }
        if self.wanted_leave {
            self.join.request_leave();
        }
        if let Some(code) = self.join.pump(join, Instant::now())
            && self.supervisor.join_refused(view, code)
        {
            self.finish(port);
        }
        // a run that ended since its tick asked for a command sends nothing
        if !self.supervisor.is_active() {
            self.command.clear();
        } else if let Err(error) = self.command.pump(command) {
            self.supervisor.abort(error.why());
            self.finish(port);
        }
        *status.lock().unwrap_or_else(PoisonError::into_inner) = self.supervisor.status(view);
    }

    // disable: the active run ends aborted, then the held input and the lease are released
    pub fn end_for_disable<P: ManagerPort>(&mut self, port: Option<&P>, status: &Mutex<RunStatus>) {
        self.end_for(port, Self::DISABLED, status);
    }

    // a cause outside the frame (the mod disabled, the lease lost, a panic): the active run ends aborted with `why`,
    // then the held input and the lease are released and the status snapshot shows the end
    pub fn end_for<P: ManagerPort>(&mut self, port: Option<&P>, why: &'static str, status: &Mutex<RunStatus>) {
        self.supervisor.abort(why);
        self.finish(port);
        let mut snapshot = status.lock().unwrap_or_else(PoisonError::into_inner);
        let view = PlayerView {
            position: snapshot.position,
            yaw: snapshot.yaw,
            pitch: snapshot.pitch,
            dimension: snapshot.dimension,
            screen: ScreenKind::InWorld,
        };
        *snapshot = self.supervisor.status(&view);
    }

    // sends the input, turning from the look `view` reads; a send the manager refuses aborts the run, or stops it
    // when the panel opened since the frame
    fn send<P: ManagerPort>(&mut self, frame: &InputFrame, port: Option<&P>, view: &PlayerView) {
        self.wanted_join.clone_from(&frame.join);
        self.wanted_leave = frame.leave;
        if let Some(text) = &frame.command
            && !self.command.request(text)
        {
            curia::warn!("a command was dropped: too many wait for a frame", { capacity: CommandService::CAPACITY });
        }
        let Some(port) = port else { return };
        if let Err(error) = self.output.apply(frame, port, Some((view.yaw, view.pitch))) {
            if port.panel_open() {
                let _ = self.supervisor.apply(Command::Stop);
            } else {
                self.supervisor.abort(error.why());
            }
        }
    }

    // a run end: every held input goes up, then the lease is given back
    fn finish<P: ManagerPort>(&mut self, port: Option<&P>) {
        if let Some(port) = port {
            self.output.end(port, &mut self.lease);
        }
        self.wanted_join = None;
        self.wanted_leave = false;
        self.command.clear();
        if let Some(result) = self.supervisor.take_end() {
            Self::log_end(&result);
        }
    }

    fn take_commands<P: ManagerPort>(&mut self, inbox: &CommandInbox, port: Option<&P>) {
        let mut commands = std::mem::take(&mut self.commands);
        let mut replies = std::mem::take(&mut self.replies);
        inbox.drain(&mut commands, &mut replies);
        for (command, reply) in commands.drain(..).zip(replies.drain(..)) {
            reply.answer(|| self.apply(command, port));
        }
        self.commands = commands;
        self.replies = replies;
    }

    // a command that starts a run or live session takes the lease first; a refused lease starts nothing, and a lease
    // taken for a command the supervisor refuses is given back
    fn apply<P: ManagerPort>(&mut self, command: Command, port: Option<&P>) -> Result<RunId, PortError> {
        let starts = match &command {
            Command::RunFile { .. } => true,
            Command::Live(action) => !matches!(action, LiveAction::Release),
            Command::Stop => false,
        };
        let mut took = false;
        if starts && !self.lease.is_held() {
            let Some(port) = port else {
                return Err(PortError::Refused(Self::NO_MANAGER.to_owned()));
            };
            self.lease.begin(port).map_err(|error| PortError::Refused(error.to_string()))?;
            took = true;
        }
        self.supervisor.apply(command).map_err(|refusal| {
            if took && let Some(port) = port {
                self.lease.end(port);
            }
            PortError::Refused(refusal.to_string())
        })
    }

    fn log_end(result: &RunResult) {
        let run = result.run.0;
        let ticks = result.ticks;
        match &result.reason {
            EndReason::Done => curia::info!("the run ended done", { run: run, ticks: ticks }),
            EndReason::Stopped => curia::info!("the run was stopped", { run: run, ticks: ticks }),
            EndReason::Failed { why, .. } => {
                curia::warn!("the run failed", { run: run, ticks: ticks, why: why.to_string() });
            }
            EndReason::Aborted { why } => {
                curia::warn!("the run was aborted", { run: run, ticks: ticks, why: why.to_string() });
            }
        }
    }
}
