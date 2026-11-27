use std::sync::Arc;

use amml_api::amml_abi::{ClientTick, GameTick, JoinServerResult, WorldLeft};
use amml_api::{ApiError, Ctx, HookOrder, JoinServerResultExt, Listener};

use crate::services::HeadlessRuntime;

// The listeners of the enabled mod and the runtime they drive. `stop` drops the listeners first (no listener runs
// after), then ends the run and closes the link and the MCP side
pub struct HeadlessListeners {
    listeners: Vec<Listener>,
    runtime: Arc<HeadlessRuntime>,
}

impl HeadlessListeners {
    // `ClientTick` (the frame), `GameTick` (the run driver's tick), `JoinServerResult` (the join answer) and
    // `WorldLeft` (the next world is a new one)
    pub fn start(ctx: &Ctx, runtime: Arc<HeadlessRuntime>) -> Result<Self, ApiError> {
        let events = ctx.events();
        let mut listeners = Vec::with_capacity(4);
        let frame = Arc::clone(&runtime);
        listeners.push(events.listen::<ClientTick>(HookOrder::NORMAL, move |_, at| {
            if let Some(client) = at.client() {
                frame.frame(&client);
            }
        })?);
        let tick = Arc::clone(&runtime);
        listeners.push(events.listen::<GameTick>(HookOrder::NORMAL, move |_, at| {
            if let Some(client) = at.client() {
                tick.game_tick(&client);
            }
        })?);
        let answer = Arc::clone(&runtime);
        listeners.push(events.listen::<JoinServerResult>(HookOrder::NORMAL, move |event, _| {
            answer.join_answered(event.outcome());
        })?);
        let left = Arc::clone(&runtime);
        listeners.push(events.listen::<WorldLeft>(HookOrder::NORMAL, move |_, _| left.world_left())?);
        Ok(Self { listeners, runtime })
    }

    pub fn runtime(&self) -> &Arc<HeadlessRuntime> {
        &self.runtime
    }

    pub fn stop(self) {
        drop(self.listeners);
        self.runtime.detach();
    }
}
