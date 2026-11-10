use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, RwLock};
use std::time::{Duration, Instant};

use amm_client::link::ApiTable;
use amm_core::errors::LinkError;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amml_game::GameEntries;
use amml_sdk::{HookHandle, ModDirs, ModRuntime};
use headless_input_core::config::HeadlessConfig;
use headless_input_core::player::{PlayerView, ScreenKind};
use headless_input_core::run::{RunDefaults, RunStatus};
use headless_input_core::structs::Capabilities;
use headless_input_mcp::{InstanceId, LegacyCopy, LegacyTokenFile};

use crate::command::{CommandClient, GameCommandClient};
use crate::game::{GameReader, HeadlessEntry};
use crate::join::{GameJoinClient, JoinClient};
use crate::services::{
    CommandInbox, HeadlessLink, HeadlessTokio, HubControl, LastView, LocalPort, NodeLauncher, PairingPrompt, RunDriver,
    TickWatch,
};
use crate::traits::ManagerPort;
use crate::types::{FrameView, LinkState};

// what the hooks share: the settings, the game client and entries, the capabilities they give, the manager link
// state, the installed hooks, the run driver and the MCP side. The mod manager is reached on the first
// ClientInstance::update after enable, when every mod of the host has loaded; `hooks` holds at most HOOK_CAPACITY
// handles. Each frame reads the screen and the local player. The client tick (`ClientInstance::onTick`) gives the run
// driver one tick for each game tick of the captured client that has a local player. A frame in a world whose player read failed gives the driver the last good view of that world
// (`last_view`); `tick_watch` logs whether the local tick is ever seen. The MCP hub reaches the game only through the
// local port: its commands wait in `inbox` for the next frame, and it reads `status`, which each frame writes. Lock
// order: `link`, then `driver`
pub struct HeadlessRuntime {
    client: AtomicUsize,
    entries: RwLock<Option<GameEntries>>,
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
    config_dir: PathBuf,
    data_dir: PathBuf,
    // `%LOCALAPPDATA%\amml\mod-data\@alaydriem\headless-input`, the same for every instance; `None` from a loader without it
    shared_data_dir: Option<PathBuf>,
    instance: InstanceId,
    tokio: Mutex<Option<HeadlessTokio>>,
    hub: Mutex<Option<HubControl<NodeLauncher>>>,
    link_state: AtomicU8,
    link: Mutex<Option<HeadlessLink>>,
    hooks: Mutex<Vec<HookHandle>>,
}

static RUNTIME: OnceLock<HeadlessRuntime> = OnceLock::new();

impl HeadlessRuntime {
    pub const HOOK_CAPACITY: usize = 8;
    // the longest the hub and the Tokio runtime take to stop in `detach`
    pub const DISABLE_BOUND: Duration = HubControl::<NodeLauncher>::SWITCH_WAIT
        .saturating_add(HubControl::<NodeLauncher>::NODE_STOP_WAIT)
        .saturating_add(HeadlessTokio::STOP_WAIT);
    // the instance folder sits above this one
    const MODS_FOLDER: &'static str = "mods";

    pub fn new(dirs: &ModDirs) -> Self {
        let driver = RunDriver::new(RunDefaults::default());
        let idle = PlayerView { position: [0.0; 3], yaw: 0.0, pitch: 0.0, dimension: None, screen: ScreenKind::Title };
        let status = driver.status(&idle);
        Self {
            client: AtomicUsize::new(0),
            entries: RwLock::new(None),
            capabilities: Arc::new(RwLock::new(Capabilities::new(false, false, false, false, false, false))),
            driver: Mutex::new(driver),
            inbox: CommandInbox::new_shared(),
            status: Arc::new(Mutex::new(status)),
            prompt: PairingPrompt::new_shared(),
            pairing_seen: AtomicBool::new(false),
            last_view: Mutex::new(LastView::new()),
            tick_watch: Mutex::new(TickWatch::new()),
            config: Arc::new(RwLock::new(HeadlessConfig::default())),
            config_dir: dirs.config_dir.clone(),
            data_dir: dirs.data_dir.clone(),
            shared_data_dir: dirs.shared_data_dir.clone(),
            instance: Self::instance_id(dirs),
            tokio: Mutex::new(None),
            hub: Mutex::new(None),
            link_state: AtomicU8::new(LinkState::Disabled.to_u8()),
            link: Mutex::new(None),
            hooks: Mutex::new(Vec::with_capacity(Self::HOOK_CAPACITY)),
        }
    }

    // the runtime of this DLL; the first call wins
    pub fn install(dirs: &ModDirs) -> &'static Self {
        RUNTIME.get_or_init(|| Self::new(dirs))
    }

    pub fn get() -> Option<&'static Self> {
        RUNTIME.get()
    }

    // the AMML instance folder name, the parent of the `mods` folder the mod sits in; `instance-<pid>` when the mod
    // folder is not inside one
    pub fn instance_id(dirs: &ModDirs) -> InstanceId {
        let name = dirs
            .mod_dir
            .ancestors()
            .find(|folder| folder.file_name().is_some_and(|name| name.eq_ignore_ascii_case(Self::MODS_FOLDER)))
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned());
        InstanceId::new(name.unwrap_or_else(|| format!("instance-{}", std::process::id())))
    }

    pub fn link_state(&self) -> LinkState {
        LinkState::from_u8(self.link_state.load(Ordering::SeqCst))
    }

    pub fn client(&self) -> usize {
        self.client.load(Ordering::Relaxed)
    }

    // the settings now in force
    pub fn config(&self) -> HeadlessConfig {
        self.config.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    // keeps the first non-null client; later calls change nothing. The primary client is created at start-up and
    // lives until the game closes, while split-screen clients come and go
    pub fn capture(&self, client: usize) {
        if client != 0 {
            let _ = self.client.compare_exchange(0, client, Ordering::Relaxed, Ordering::Relaxed);
        }
    }

    pub fn set_entries(&self, entries: GameEntries) {
        *self.capabilities.write().unwrap_or_else(PoisonError::into_inner) = HeadlessEntry::capabilities(&entries);
        *self.entries.write().unwrap_or_else(PoisonError::into_inner) = Some(entries);
    }

    // the player reads the resolved entries give; none before enable. A control file is loaded against them
    pub fn capabilities(&self) -> Capabilities {
        *self.capabilities.read().unwrap_or_else(PoisonError::into_inner)
    }

    // None without the entries
    pub fn with_entries<R>(&self, body: impl FnOnce(&GameEntries) -> R) -> Option<R> {
        let entries = self.entries.read().unwrap_or_else(PoisonError::into_inner);
        entries.as_ref().map(body)
    }

    // the port the hub calls this instance through; a command no frame takes within `wait` times out
    pub fn local_port(&self, wait: Duration) -> Arc<LocalPort> {
        Arc::new(LocalPort::new(
            Arc::clone(&self.inbox),
            Arc::clone(&self.status),
            Arc::clone(&self.capabilities),
            Arc::clone(&self.config),
            Arc::clone(&self.prompt),
            self.data_dir.clone(),
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

    // enable: the runtime starts Disabled, so a tick before this call does nothing. The settings start from their
    // defaults, so the mod works without a manager, and the next tick reaches the manager
    pub fn enable_link(&self) {
        self.inbox.open();
        self.prompt.reopen();
        self.pairing_seen.store(false, Ordering::SeqCst);
        self.apply_config(HeadlessConfig::default());
        self.link_state.store(LinkState::Waiting.to_u8(), Ordering::SeqCst);
    }

    // enable, after the hooks: the Tokio runtime and the hub control. Without the runtime MCP stays off, with one
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

    /// Once per `ClientInstance::update` on the game thread, after the original: the manager link, the frame read
    /// and the run driver's frame, then the hub follows the settings.
    ///
    /// # Safety
    /// `client` is the live `ClientInstance` the game passed to its update, and the call is on the game thread.
    #[allow(unsafe_code)]
    pub unsafe fn tick(&self, client: usize) {
        self.guarded(|| {
            self.capture(client);
            self.tick_with(ApiTable::resolve);
            // SAFETY: forwarded to the caller
            unsafe { self.frame(client) };
            self.follow_hub();
        });
    }

    // runs one hook body. A panic in it does not reach the game: the active run ends (held input released, then the
    // lease), one line is logged, and the runtime turns Disabled, so the hooks do nothing until the mod is enabled
    // again
    pub fn guarded(&self, body: impl FnOnce()) {
        if std::panic::catch_unwind(AssertUnwindSafe(body)).is_err() {
            self.after_panic();
        }
    }

    // the first panic only: a runtime already Disabled has nothing left to end
    fn after_panic(&self) {
        let disabled = LinkState::Disabled.to_u8();
        if self.link_state.swap(disabled, Ordering::SeqCst) == disabled {
            return;
        }
        curia::error!(
            "a hook body of @alaydriem/headless-input panicked; the run ended and the mod stays idle until it is enabled again"
        );
        self.inbox.close();
        self.prompt.cancel();
        let released = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let slot = self.link.lock().unwrap_or_else(PoisonError::into_inner);
            self.driver().end_for(slot.as_ref(), RunDriver::PANICKED, &self.status);
        }));
        if released.is_err() {
            curia::error!("the input could not be released after the panic; disable the mod in the manager panel");
        }
    }

    /// On each call of the client tick hook, on the game thread, after the original. `callbacks` is the
    /// `GameCallbacks` subobject the game passed. Only a tick of the client this runtime captured, with a local
    /// player, reaches the run driver; a tick whose read fails is skipped.
    ///
    /// # Safety
    /// The call is on the game thread, and `callbacks` is the live `GameCallbacks` subobject of a `ClientInstance`.
    #[allow(unsafe_code)]
    pub unsafe fn tick_game(&self, callbacks: usize) {
        self.guarded(|| {
            // SAFETY: forwarded to the caller
            unsafe { self.game_tick(callbacks) };
        });
    }

    #[allow(unsafe_code)]
    unsafe fn game_tick(&self, callbacks: usize) {
        if self.link_state() == LinkState::Disabled {
            return;
        }
        let capabilities = self.capabilities();
        self.with_entries(|entries| {
            let offset = usize::try_from(entries.get(HeadlessEntry::CLIENT_INSTANCE_GAME_CALLBACKS_OFFSET)?).ok()?;
            let client = callbacks.checked_sub(offset)?;
            if client != self.client() {
                return None;
            }
            let reader = GameReader::new(entries, capabilities);
            // SAFETY: `client` is the captured ClientInstance, live (the caller), on the game thread
            let view = unsafe { reader.read(client) };
            if view.is_some() && self.tick_watch.lock().unwrap_or_else(PoisonError::into_inner).on_local_tick() {
                curia::info!("the game tick reached the local player in this world");
            }
            self.drive_linked_tick(view.as_ref());
            Some(())
        });
    }

    // one game tick for the run driver, through the manager link. A tick with no local player (None) does not advance
    pub fn drive_linked_tick(&self, view: Option<&PlayerView>) {
        let link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        self.drive_tick(view, link.as_ref());
    }

    // one game frame for the run driver, through the manager link
    pub fn drive_linked_frame<J: JoinClient, C: CommandClient>(&self, frame: &FrameView, join: &J, command: &C) {
        let link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        self.drive_frame(frame, link.as_ref(), join, command);
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

    // reads the frame and gives it to the run driver; nothing while the runtime is disabled
    #[allow(unsafe_code)]
    unsafe fn frame(&self, client: usize) {
        if self.link_state() == LinkState::Disabled {
            return;
        }
        let capabilities = self.capabilities();
        let driven = self.with_entries(|entries| {
            let reader = GameReader::new(entries, capabilities);
            let mut last_view = self.last_view.lock().unwrap_or_else(PoisonError::into_inner);
            // SAFETY: `client` is the live ClientInstance of this update (the caller), on the game thread
            let mut frame = unsafe { reader.frame(client, last_view.screen()) }?;
            // SAFETY: as above
            let game_client = unsafe { reader.client(client) }?;
            // SAFETY: as above
            let command_client = unsafe { reader.client(client) }?;
            let command = GameCommandClient::new(command_client, entries);
            let join = GameJoinClient::new(game_client, entries);
            if self.tick_watch.lock().unwrap_or_else(PoisonError::into_inner).on_frame(frame.in_world) {
                curia::warn!("no game tick reached the local player in this world; runs cannot advance", {
                    frames: TickWatch::FRAMES_BEFORE_WARNING
                });
            }
            last_view.settle(&mut frame);
            drop(last_view);
            self.drive_linked_frame(&frame, &join, &command);
            Some(())
        });
        if driven.flatten().is_none() {
            self.forget_world();
        }
    }

    // reaches the manager when the link is Waiting, then handles its events
    pub fn tick_with(&self, table: impl FnOnce() -> Result<ApiTable, LinkError>) {
        if self.link_state() == LinkState::Waiting {
            self.attach_with(table());
        }
        match self.link_state() {
            LinkState::Attached => self.run(),
            LinkState::Off | LinkState::Waiting | LinkState::Disabled => {}
        }
    }

    // registers with the manager the table belongs to; on failure the settings keep their defaults and one line is
    // logged. Only a Waiting runtime takes the result: if disable ran meanwhile, a new link is dropped again
    fn attach_with(&self, table: Result<ApiTable, LinkError>) {
        let result = table.and_then(|table| HeadlessLink::new(table, &self.config_dir));
        let (state, link) = match result {
            Ok(link) => (LinkState::Attached, Some(link)),
            Err(error) => {
                Self::log_link_error(&error);
                (LinkState::Off, None)
            }
        };
        let waiting = LinkState::Waiting.to_u8();
        let mut slot = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        if self.link_state.compare_exchange(waiting, state.to_u8(), Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }
        if let Some(mut link) = link {
            self.apply_config(link.config().clone());
            // the value at attach is the starting point: it allows nothing, and is turned off
            self.pairing_seen.store(link.config().mcp_allow_pairing, Ordering::SeqCst);
            self.follow_pairing_switch(&mut link);
            *slot = Some(link);
            curia::info!("@alaydriem/headless-input attached to the mod manager");
        }
    }

    // disable: the port callers are answered at once (queued commands refused, a pairing question denied), the run
    // ends and the held input and the lease are released through the link, then the hub and the Tokio runtime stop
    // with the link mutex free, then the link is dropped and the mod unregisters, and the local player is forgotten,
    // all before the hooks come out. The hub and Tokio stop within DISABLE_BOUND
    pub fn detach(&self) {
        self.link_state.store(LinkState::Disabled.to_u8(), Ordering::SeqCst);
        self.inbox.close();
        self.prompt.cancel();
        let link = {
            let mut slot = self.link.lock().unwrap_or_else(PoisonError::into_inner);
            self.release_then(slot.as_ref(), || {});
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
        self.shared_data_dir.clone().unwrap_or_else(|| self.data_dir.clone())
    }

    // copies the token file of older builds into `tokens` once, so the clients paired before keep their tokens
    fn adopt_legacy_tokens(&self, tokens: &Path) {
        if self.shared_data_dir.is_none() {
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

    // a frame with no client read, and disable: no held view, and the next world is a new one
    fn forget_world(&self) {
        self.last_view.lock().unwrap_or_else(PoisonError::into_inner).clear();
        *self.tick_watch.lock().unwrap_or_else(PoisonError::into_inner) = TickWatch::new();
    }

    // keeps an installed hook; false, keeping nothing, when HOOK_CAPACITY hooks are already kept
    pub fn keep_hook(&self, hook: HookHandle) -> bool {
        let mut hooks = self.hooks.lock().unwrap_or_else(PoisonError::into_inner);
        if hooks.len() >= Self::HOOK_CAPACITY {
            return false;
        }
        hooks.push(hook);
        true
    }

    // the installed hooks, newest first, leaving none behind
    pub fn take_hooks(&self) -> Vec<HookHandle> {
        let mut hooks = self.hooks.lock().unwrap_or_else(PoisonError::into_inner);
        let mut taken = std::mem::take(&mut *hooks);
        taken.reverse();
        taken
    }

    // the manager events: a settings change reloads the settings (the pairing switch answers the pairing question), a
    // release of the fake input drops the output record. The `Input` lease suspended or revoked by another mod
    // aborts the run as INPUT_TAKEN, and the mod disabled in the manager aborts it as DISABLED; either ends it with
    // the held input released, then the lease
    fn run(&self) {
        let mut slot = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(link) = slot.as_mut() else { return };
        let mut changed = false;
        let mut released = false;
        let mut input_taken = false;
        let mut disabled = false;
        for event in link.drain() {
            match event {
                ModEvent::SettingsChanged { .. } => changed = true,
                ModEvent::FakeKeysReleased => released = true,
                ModEvent::Suspended { capability: Capability::Input, .. }
                | ModEvent::Revoked { capability: Capability::Input, .. } => input_taken = true,
                ModEvent::Disabled => disabled = true,
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
        self.follow_pairing_switch(link);
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

    fn log_link_error(error: &LinkError) {
        let Some(runtime) = ModRuntime::get() else { return };
        match error {
            LinkError::ManagerMissing | LinkError::AbiMismatch { .. } => {
                runtime
                    .logger()
                    .error("@alaydriem/headless-input needs @amml/modmanager 26.62 or newer; it cannot block input");
            }
            other => {
                runtime.logger().error(&format!(
                    "@amml/modmanager refused @alaydriem/headless-input ({other}); it cannot block input"
                ));
            }
        }
    }
}
