use std::sync::{Arc, Mutex, PoisonError};

use amm_client::link::ManagerLink;
use amml_api::amml_abi::{
    ClientTick, GameTick, InjectedInputReleased, InputLeaseLost, InputLeaseResumed, InputLeaseSuspended,
    JoinServerResult, ModsLoaded, ProviderGone, ProviderReady, ScreenshotTaken, ScreenshotWritten, WorldLeft,
};
use amml_api::{AmmlMod, ApiError, Area, Ctx, DisableCtx, Uses};
use headless_input_core::structs::Capabilities;

use crate::services::{HeadlessDescriptor, HeadlessLink, HeadlessListeners, HeadlessRuntime};
use crate::types::RuntimeFolders;

// what the enabled mod owns. `disable` takes it, which drops the listeners, then ends the run, stops the hub and
// closes the link
static RUNNING: Mutex<Option<HeadlessListeners>> = Mutex::new(None);

// the AMML entry point: enable makes the runtime, starts the manager link, the listeners and the MCP side; disable
// removes them
pub struct HeadlessMod;

impl HeadlessMod {
    // the id as the loader names the mod; a test checks it against `AmmlMod::id`
    pub const MODULE: &'static str = "@alaydriem/headless-input";

    // the reads and calls this game serves, from the loader's areas
    pub fn capabilities(ctx: &Ctx) -> Capabilities {
        let has = |areas: &[Area]| areas.iter().all(|area| ctx.supports(*area));
        let actor = has(&[Area::ACTOR_READ.at(2)]);
        Capabilities::new(
            actor,
            actor,
            actor,
            has(&[Area::CLIENT_JOIN_SERVER, Area::CLIENT_CAN_JOIN, Area::CLIENT_LEAVE_WORLD]),
            has(&[Area::CLIENT_LEAVE_WORLD]),
            has(&[Area::CLIENT_SEND_COMMAND]),
            has(&[Area::CLIENT_START_LOCAL_WORLD, Area::CLIENT_CAN_JOIN, Area::CLIENT_LEAVE_WORLD]),
        )
    }
}

impl AmmlMod for HeadlessMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "headless-input";
    const USES: Uses = amml_api::amml_uses! {
        feature "core" {
            required ClientTick;
            required GameTick;
            required WorldLeft;
            required ProviderReady;
            required ProviderGone;
            required ModsLoaded;
            required area Area::CLIENT_READ;
            required area Area::CLIENT_TOP_SCREEN;
        }
        feature "input" {
            required InputLeaseSuspended;
            required InputLeaseResumed;
            required InputLeaseLost;
            required InjectedInputReleased;
            required area Area::INPUT_LEASE;
            required area Area::MOVE;
            required area Area::KEY_INJECT;
            required area Area::MOUSE_INJECT;
            required area Area::TURN;
        }
        feature "player" {
            required area Area::ACTOR_READ.at(2);
        }
        feature "hotbar" {
            required area Area::HUD_SELECT_HOTBAR;
        }
        feature "join" {
            required JoinServerResult;
            required area Area::CLIENT_JOIN_SERVER;
            required area Area::CLIENT_CAN_JOIN;
            required area Area::CLIENT_LEAVE_WORLD;
            required area Area::CLIENT_START_LOCAL_WORLD;
            required area Area::CLIENT_OPEN_MENU;
        }
        feature "command" {
            required area Area::CLIENT_SEND_COMMAND;
        }
        feature "screenshot" {
            required ScreenshotTaken;
            required ScreenshotWritten;
            required area Area::OVERLAY_SCREENSHOT;
        }
    };

    fn enable(ctx: &Ctx) -> Result<(), ApiError> {
        let log = ctx.log();
        let runtime = Arc::new(HeadlessRuntime::new(RuntimeFolders::from_dirs(&ctx.dirs())));
        runtime.set_capabilities(Self::capabilities(ctx));
        let screenshots = ctx.supports(Area::OVERLAY_SCREENSHOT);
        let link = match ManagerLink::start_without_frame(ctx, HeadlessDescriptor::TEXT) {
            Ok(link) => Some(HeadlessLink::new(link, screenshots)),
            Err(error) => {
                let error = error.to_string();
                log.error("the mod manager link did not start; runs that need input are refused", &[("error", &error)]);
                None
            }
        };
        runtime.enable(link, screenshots);
        let listeners = HeadlessListeners::start(ctx, Arc::clone(&runtime)).inspect_err(|error| {
            log.error("headless-input off", &[("error", &format!("{error:?}"))]);
        })?;
        runtime.start_services();
        *RUNNING.lock().unwrap_or_else(PoisonError::into_inner) = Some(listeners);
        Ok(())
    }

    fn disable(_ctx: &Ctx, _how: &DisableCtx) {
        let running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(listeners) = running {
            listeners.stop();
        }
    }
}
