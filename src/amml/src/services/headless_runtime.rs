use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::{Duration, Instant};

use amm_core::structs::event::ModEvent;
use amml_api::{Client, JoinOutcome};
use headless_input_core::config::HeadlessConfig;
use headless_input_core::player::{PlayerView, ScreenKind};
use headless_input_core::run::{RunDefaults, RunStatus};
use headless_input_core::structs::{Capabilities, ScreenshotSlot};
use headless_input_mcp::{InstanceId, LegacyCopy, LegacyTokenFile};

use crate::command::{CommandClient, GameCommandClient};
use crate::game::FrameReader;
use crate::join::{GameJoinClient, JoinClient, JoinCodes};
use crate::services::{
    CommandInbox, GamePort, HeadlessLink, HeadlessTokio, HubControl, LastView, LocalPort, NodeLauncher, PairingPrompt,
    RunDriver, ScreenshotDesk, ScreenshotFolder, TickWatch, WorldFolderReader,
};
use crate::traits::ManagerPort;
use crate::types::{FrameView, LinkState, RuntimeFolders};

// What the listeners share: the settings, the capabilities the loader's areas give, the manager link and its state,
// the run driver and the MCP side. The link starts in enable and connects at the manager's `ProviderReady`; the first
// frame that finds it connected attaches (the settings are read, one line logged). Each frame reads the screen and the
// local player through the loader. Each game tick of the primary client with a local player gives the run driver one
// tick. A frame in a world whose player read failed gives the driver the last good view of that world (`last_view`);
// `tick_watch` logs whether the local tick is ever seen. The MCP hub reaches the game only through the local port: its
// commands wait in `inbox` for the next frame, and it reads `status`, which each frame writes. Lock order: `link`, then
// `driver`, then the screenshot desk. The port thread takes only the desk
pub struct HeadlessRuntime {
    capabilities: Arc<RwLock<Capabilities>>,
    driver: Mutex<RunDriver>,
    inbox: Arc<CommandInbox>,
    status: Arc<Mutex<RunStatus>>,
    prompt: Arc<PairingPrompt>,
    // mcp.allow_pairing as the last read of the settings saw it
    pairing_seen: AtomicBool,
    last_view: Mutex<LastView>,
    tick_watch: Mutex<TickWatch>,
    config: Arc<RwLock<HeadlessConfig>>,
    folders: RuntimeFolders,
    instance: InstanceId,
    tokio: Mutex<Option<HeadlessTokio>>,
    hub: Mutex<Option<HubControl<NodeLauncher>>>,
    link_state: Arc<AtomicU8>,
    desk: Arc<ScreenshotDesk>,
    link: Mutex<Option<HeadlessLink>>,
    join_codes: JoinCodes,
}

impl HeadlessRuntime {
    // the longest the hub and the Tokio runtime take to stop in `detach`
    pub const DISABLE_BOUND: Duration = HubControl::<NodeLauncher>::SWITCH_WAIT
        .saturating_add(HubControl::<NodeLauncher>::NODE_STOP_WAIT)
        .saturating_add(HeadlessTokio::STOP_WAIT);

    pub fn new(folders: RuntimeFolders) -> Self {
        let driver = RunDriver::new(RunDefaults::default());
        let idle = PlayerView { position: [0.0; 3], yaw: 0.0, pitch: 0.0, dimension: None, screen: ScreenKind::Title };
        let status = driver.status(&idle);
        let instance = Self::instance_id(&folders);
        Self {
            capabilities: Arc::new(RwLock::new(Capabilities::new(false, false, false, false, false, false, false))),
            driver: Mutex::new(driver),
            inbox: CommandInbox::new_shared(),
            status: Arc::new(Mutex::new(status)),
            prompt: PairingPrompt::new_shared(),
            pairing_seen: AtomicBool::new(false),
            last_view: Mutex::new(LastView::new()),
            tick_watch: Mutex::new(TickWatch::new()),
            config: Arc::new(RwLock::new(HeadlessConfig::default())),
            folders,
            instance,
            tokio: Mutex::new(None),
            hub: Mutex::new(None),
            link_state: Arc::new(AtomicU8::new(LinkState::Disabled.to_u8())),
            desk: ScreenshotDesk::new_shared(ScreenshotSlot::TIMEOUT),
            link: Mutex::new(None),
            join_codes: JoinCodes::default(),
        }
    }

    // the AMML instance folder name; `instance-<pid>` when the loader gives no instance folder
    pub fn instance_id(folders: &RuntimeFolders) -> InstanceId {
        let name =
            folders.instance_dir.as_deref().and_then(Path::file_name).map(|name| name.to_string_lossy().into_owned());
        InstanceId::new(name.unwrap_or_else(|| format!("instance-{}", std::process::id())))
    }

    pub fn link_state(&self) -> LinkState {
        LinkState::from_u8(self.link_state.load(Ordering::SeqCst))
    }

    // the settings now in force
    pub fn config(&self) -> HeadlessConfig {
        self.config.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    // the reads and calls the loader serves on this game; a control file is loaded against them
    pub fn set_capabilities(&self, capabilities: Capabilities) {
        *self.capabilities.write().unwrap_or_else(PoisonError::into_inner) = capabilities;
    }

    pub fn capabilities(&self) -> Capabilities {
        *self.capabilities.read().unwrap_or_else(PoisonError::into_inner)
    }

    // the join answers the `JoinServerResult` listener stores for the join service
    pub fn join_codes(&self) -> &JoinCodes {
        &self.join_codes
    }

    // the port the hub calls this instance through; a command no frame takes within `wait` times out
    pub fn local_port(&self, wait: Duration) -> Arc<LocalPort> {
        let fallback_root = self
            .folders
            .game_data_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()));
        Arc::new(LocalPort::new(
            Arc::clone(&self.inbox),
            Arc::clone(&self.status),
            Arc::clone(&self.capabilities),
            Arc::clone(&self.config),
            Arc::clone(&self.prompt),
            self.folders.data_dir.clone(),
            WorldFolderReader::new(self.folders.instance_dir.clone().unwrap_or_default(), fallback_root),
            Arc::clone(&self.desk),
            ScreenshotFolder::new(self.folders.data_dir.join(LocalPort::SCREENSHOT_FOLDER)),
            Arc::clone(&self.link_state),
            wait,
        ))
    }

    // whether a pairing question waits for the switch
    pub fn pairing_waiting(&self) -> bool {
        self.prompt.is_asking()
    }

    // the commands waiting for the next frame
    pub fn queued(&self) -> usize {
        self.inbox.len()
    }

    // enable: the settings start from their defaults, so the mod works without a manager. With a link the runtime
    // waits for it to connect; without one (no manager link could start) it is Off. `screenshots` is whether the loader
    // takes screenshots
    pub fn enable(&self, link: Option<HeadlessLink>, screenshots: bool) {
        self.inbox.open();
        self.desk.open();
        self.desk.set_supported(screenshots);
        self.prompt.reopen();
        self.pairing_seen.store(false, Ordering::SeqCst);
        self.apply_config(HeadlessConfig::default());
        let state = if link.is_some() { LinkState::Waiting } else { LinkState::Off };
        *self.link.lock().unwrap_or_else(PoisonError::into_inner) = link;
        self.link_state.store(state.to_u8(), Ordering::SeqCst);
    }

    // enable, after the listeners: the Tokio runtime and the hub control. Without the runtime MCP stays off, with one
    // error line
    pub fn start_services(&self) {
        let tokio = match HeadlessTokio::start() {
            Ok(tokio) => tokio,
            Err(error) => {
                curia::error!("the MCP runtime did not start; MCP stays off", { error: error.to_string() });
                return;
            }
        };
        let tokens = self.token_folder();
        self.adopt_legacy_tokens(&tokens);
        let launcher =
            NodeLauncher::new(tokio.handle(), self.local_port(LocalPort::WAIT), self.instance.clone(), &tokens);
        let hub = HubControl::new(tokio.handle(), launcher);
        *self.hub.lock().unwrap_or_else(PoisonError::into_inner) = Some(hub);
        *self.tokio.lock().unwrap_or_else(PoisonError::into_inner) = Some(tokio);
    }

    /// One `ClientTick` of the primary client, on the game thread: the manager link, the frame read and the run
    /// driver's frame, then the hub follows the settings.
    pub fn frame(&self, client: &Client<'_>) {
        if self.link_state() == LinkState::Disabled {
            return;
        }
        self.follow_link();
        let reader = FrameReader::new(self.capabilities());
        let mut last_view = self.last_view.lock().unwrap_or_else(PoisonError::into_inner);
        let mut frame = reader.frame(client, last_view.screen());
        if self.tick_watch.lock().unwrap_or_else(PoisonError::into_inner).on_frame(frame.in_world) {
            curia::warn!("no game tick reached the local player in this world; runs cannot advance", {
                frames: TickWatch::FRAMES_BEFORE_WARNING
            });
        }
        last_view.settle(&mut frame);
        drop(last_view);
        let join = GameJoinClient::new(client, &self.join_codes);
        let command = GameCommandClient::new(client);
        let link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        let port = link.as_ref().map(|link| GamePort::new(link, Some(client)));
        self.drive_frame(&frame, port.as_ref(), &join, &command);
        drop(link);
        self.follow_hub();
    }

    /// One `GameTick` of the primary client, on the game thread: a tick with a local player gives the run driver one
    /// tick; a tick whose read fails is skipped.
    pub fn game_tick(&self, client: &Client<'_>) {
        if self.link_state() == LinkState::Disabled {
            return;
        }
        let view = FrameReader::new(self.capabilities()).read(client);
        if view.is_some() && self.tick_watch.lock().unwrap_or_else(PoisonError::into_inner).on_local_tick() {
            curia::info!("the game tick reached the local player in this world");
        }
        let link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        let port = link.as_ref().map(|link| GamePort::new(link, Some(client)));
        self.drive_tick(view.as_ref(), port.as_ref());
    }

    /// The game answered a server join (`JoinServerResult`).
    pub fn join_answered(&self, outcome: JoinOutcome) {
        self.join_codes.answer(outcome);
    }

    /// The world was left (`WorldLeft`): no held view, and the next world is a new one.
    pub fn world_left(&self) {
        self.forget_world();
    }

    // one game tick for the run driver; a tick with no local player (None) does not advance
    pub fn drive_tick<P: ManagerPort>(&self, view: Option<&PlayerView>, port: Option<&P>) {
        if let Some(view) = view {
            self.driver().tick(view, port);
        }
    }

    // one game frame for the run driver: the supervisor's frame, the queued commands, the join and the status
    pub fn drive_frame<P: ManagerPort, J: JoinClient, C: CommandClient>(
        &self,
        frame: &FrameView,
        port: Option<&P>,
        join: &J,
        command: &C,
    ) {
        self.driver().frame(frame, port, join, command, &self.inbox, &self.status);
    }

    // one game tick for the run driver, through the manager link alone (no hotbar select)
    pub fn drive_linked_tick(&self, view: Option<&PlayerView>) {
        let link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        self.drive_tick(view, link.as_ref());
    }

    // one game frame for the run driver, through the manager link alone (no hotbar select)
    pub fn drive_linked_frame<J: JoinClient, C: CommandClient>(&self, frame: &FrameView, join: &J, command: &C) {
        let link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        self.drive_frame(frame, link.as_ref(), join, command);
    }

    // attaches when the link connected, then handles its events; the manager gone turns the link Off
    pub fn follow_link(&self) {
        let mut slot = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(link) = slot.as_mut() else { return };
        match self.link_state() {
            LinkState::Waiting if link.is_connected() => {
                self.apply_config(link.reload().clone());
                // the value at attach is the starting point: it allows nothing, and is turned off
                self.pairing_seen.store(link.config().mcp_allow_pairing, Ordering::SeqCst);
                self.follow_pairing_switch(link);
                self.link_state.store(LinkState::Attached.to_u8(), Ordering::SeqCst);
                curia::info!("@alaydriem/headless-input attached to the mod manager");
            }
            LinkState::Waiting | LinkState::Off | LinkState::Disabled => {}
            LinkState::Attached => {}
        }
        let events = link.drain().to_vec();
        self.apply_link_events(link, &events);
    }

    // the link events of one frame, given by a test without a manager
    pub fn handle_link_events(&self, events: &[ModEvent]) {
        let mut slot = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(link) = slot.as_mut() {
            self.apply_link_events(link, events);
        }
    }

    // disable: the port callers are answered at once (queued commands refused, a pairing question denied), the run
    // ends and the held input and the lease are released through the link, then the hub and the Tokio runtime stop
    // with the link mutex free, then the link is dropped. The hub and Tokio stop within DISABLE_BOUND
    pub fn detach(&self) {
        self.link_state.store(LinkState::Disabled.to_u8(), Ordering::SeqCst);
        self.inbox.close();
        self.desk.close();
        self.prompt.cancel();
        let link = {
            let mut slot = self.link.lock().unwrap_or_else(PoisonError::into_inner);
            self.release_then(slot.as_ref(), || {});
            if let Some(link) = slot.as_ref() {
                self.desk.pump(link);
            }
            slot.take()
        };
        self.stop_services();
        drop(link);
        self.forget_world();
    }

    // ends the active run as the mod is disabled and releases the held input, then the lease, through `port`; then
    // `after` runs
    pub fn release_then<P: ManagerPort>(&self, port: Option<&P>, after: impl FnOnce()) {
        self.driver().end_for_disable(port, &self.status);
        after();
    }

    // the hub first, so no new command arrives, then the runtime it ran on
    fn stop_services(&self) {
        let hub = self.hub.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(mut hub) = hub {
            hub.stop();
        }
        let tokio = self.tokio.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(tokio) = tokio {
            tokio.stop();
        }
    }

    // the folder of the token file every instance shares: the shared data folder the loader gives the mod, or the mod
    // data folder of this instance when the loader gives none
    pub fn token_folder(&self) -> PathBuf {
        self.folders.shared_data_dir.clone().unwrap_or_else(|| self.folders.data_dir.clone())
    }

    // copies the token file of older builds into `tokens` once, so the clients paired before keep their tokens
    fn adopt_legacy_tokens(&self, tokens: &Path) {
        if self.folders.shared_data_dir.is_none() {
            curia::warn!("shared data folder not available, MCP tokens stay in this instance", {
                folder: tokens.display().to_string()
            });
            return;
        }
        let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
            return;
        };
        let legacy = LegacyTokenFile::folder_in(Path::new(&local_app_data));
        match LegacyTokenFile::copy_once(&legacy, tokens) {
            Ok(LegacyCopy::Copied) => curia::info!("MCP token file copied from the old folder", {
                from: legacy.display().to_string(),
                to: tokens.display().to_string()
            }),
            Ok(LegacyCopy::AlreadyPresent | LegacyCopy::NoLegacyFile) => {}
            Err(error) => curia::warn!("MCP token file not copied from the old folder; clients pair again", {
                from: legacy.display().to_string(),
                error: error.to_string()
            }),
        }
    }

    // once a frame: the hub follows mcp.enabled and mcp.port. Never blocks: a hub control another thread holds is left
    // for the next frame
    fn follow_hub(&self) {
        if self.link_state() == LinkState::Disabled {
            return;
        }
        let (enabled, port) = {
            let config = self.config.read().unwrap_or_else(PoisonError::into_inner);
            (config.mcp_enabled, config.mcp_port)
        };
        let Ok(mut hub) = self.hub.try_lock() else { return };
        if let Some(hub) = hub.as_mut() {
            hub.follow(enabled, port, Instant::now());
        }
    }

    // a world leave and disable: no held view, and the next world is a new one
    fn forget_world(&self) {
        self.last_view.lock().unwrap_or_else(PoisonError::into_inner).clear();
        *self.tick_watch.lock().unwrap_or_else(PoisonError::into_inner) = TickWatch::new();
    }

    // the link events: a settings change reloads the settings (the pairing switch answers the pairing question), a
    // release of the injected input drops the output record. The input lease suspended or lost aborts the run as
    // INPUT_TAKEN, and the mod disabled in the manager aborts it as DISABLED; either ends it with the held input
    // released, then the lease. The manager gone turns the link Off
    fn apply_link_events(&self, link: &mut HeadlessLink, events: &[ModEvent]) {
        let mut changed = false;
        let mut released = false;
        let mut input_taken = false;
        let mut disabled = false;
        let mut gone = false;
        for event in events {
            self.desk.on_event(event);
            match event {
                ModEvent::SettingsChanged { .. } => changed = true,
                ModEvent::FakeKeysReleased { .. } => released = true,
                ModEvent::InputLeaseSuspended | ModEvent::InputLeaseLost => input_taken = true,
                ModEvent::Disabled => disabled = true,
                ModEvent::ManagerGone => gone = true,
                _ => {}
            }
        }
        if released {
            self.driver().forget_output();
        }
        let lost = if disabled { Some(RunDriver::DISABLED) } else { input_taken.then_some(RunDriver::INPUT_TAKEN) };
        if let Some(why) = lost {
            self.driver().end_for(Some(&*link), why, &self.status);
        }
        if changed {
            let config = link.reload().clone();
            self.apply_config(config);
        }
        self.desk.pump(&*link);
        self.follow_pairing_switch(link);
        if gone && self.link_state() == LinkState::Attached {
            self.link_state.store(LinkState::Off.to_u8(), Ordering::SeqCst);
            curia::warn!("the mod manager is gone; runs that need input are refused");
        }
    }

    // mcp.allow_pairing: a waiting question is allowed only when the value turned on since the last read. An on is
    // turned off at once; the document is read again, so a reset that did not apply leaves the value seen as on and
    // the next unrelated settings change allows nothing
    fn follow_pairing_switch(&self, link: &mut HeadlessLink) {
        let on = self.config.read().unwrap_or_else(PoisonError::into_inner).mcp_allow_pairing;
        let was_on = self.pairing_seen.swap(on, Ordering::SeqCst);
        if !on {
            return;
        }
        if !was_on {
            self.prompt.allow();
        }
        link.clear_pairing_switch();
        let config = link.reload().clone();
        self.pairing_seen.store(config.mcp_allow_pairing, Ordering::SeqCst);
        self.apply_config(config);
    }

    fn apply_config(&self, config: HeadlessConfig) {
        self.driver().set_defaults(RunDefaults::new(&config));
        *self.config.write().unwrap_or_else(PoisonError::into_inner) = config;
    }

    fn driver(&self) -> MutexGuard<'_, RunDriver> {
        self.driver.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
