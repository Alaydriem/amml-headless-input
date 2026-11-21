use std::path::Path;

use amm_client::link::{LinkState, ManagerLink, PointerOp};
use amm_core::structs::capture::ScreenshotLayers;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amm_core::structs::lease::AcquireOutcome;
use amml_api::{KeyCode, MoveIntent};
use headless_input_core::config::HeadlessConfig;
use headless_input_core::structs::ScreenshotLayer;

use crate::errors::{LeaseError, OutputError};
use crate::services::{RawSettings, ScreenshotDesk, SettingsReader};
use crate::traits::{ManagerPort, ScreenshotPort};

// The link to the mod manager (started in enable, connected at the manager's `ProviderReady`) and the settings read
// from it; `events` holds at most MAX_EVENTS_PER_FRAME and is cleared at the start of every drain. Whether the loader
// takes screenshots is read once at enable
pub struct HeadlessLink {
    link: ManagerLink,
    config: HeadlessConfig,
    events: Vec<ModEvent>,
    screenshots: bool,
}

impl HeadlessLink {
    pub const MAX_EVENTS_PER_FRAME: usize = 64;
    // the setting that allows the MCP pairing request that waits
    pub const PAIRING_SETTING: &'static str = "mcp.allow_pairing";
    // the holder named when the manager denies the lease and does not say who holds it
    pub const UNKNOWN_HOLDER: &'static str = "another mod";

    pub fn new(link: ManagerLink, screenshots: bool) -> Self {
        Self {
            link,
            config: HeadlessConfig::default(),
            events: Vec::with_capacity(Self::MAX_EVENTS_PER_FRAME),
            screenshots,
        }
    }

    // whether the link reached the manager
    pub fn is_connected(&self) -> bool {
        self.link.state() == LinkState::Connected
    }

    pub fn config(&self) -> &HeadlessConfig {
        &self.config
    }

    // reads the settings again
    pub fn reload(&mut self) -> &HeadlessConfig {
        self.config = self.read_settings();
        &self.config
    }

    // the queued manager and loader events the link passes on, at most MAX_EVENTS_PER_FRAME; the rest wait for the next
    // frame. The slice is valid until the next drain
    pub fn drain(&mut self) -> &[ModEvent] {
        self.events.clear();
        self.events.extend(self.link.events().take(Self::MAX_EVENTS_PER_FRAME));
        &self.events
    }

    // sets mcp.allow_pairing back to false; the manager sends the mod no event for its own write
    pub fn clear_pairing_switch(&self) {
        if let Err(error) = self.link.set_setting(Self::PAIRING_SETTING, &hcl::Value::Bool(false)) {
            curia::warn!("the pairing switch could not be turned off", { error: error.to_string() });
        }
    }

    fn read_settings(&self) -> HeadlessConfig {
        match self.link.config::<RawSettings>() {
            Ok(raw) => SettingsReader::convert(&raw),
            Err(error) => {
                curia::warn!("config could not be read; using the defaults", { error: error.to_string() });
                HeadlessConfig::default()
            }
        }
    }
}

impl ManagerPort for HeadlessLink {
    fn acquire_input(&self) -> Result<(), LeaseError> {
        match self.link.acquire(Capability::Input).map_err(LeaseError::Link)? {
            AcquireOutcome::Granted | AcquireOutcome::Preempted => Ok(()),
            AcquireOutcome::Denied { .. } => {
                let holder = self.link.holder(Capability::Input).unwrap_or_else(|| Self::UNKNOWN_HOLDER.to_string());
                Err(LeaseError::Refused { holder })
            }
        }
    }

    fn release_input(&self) {
        self.link.release(Capability::Input);
    }

    // a virtual-key code of 1 to FEh; another code is not a key
    fn send_key(&self, vk: u32, down: bool) -> Result<(), OutputError> {
        let key = u16::try_from(vk).ok().filter(|code| (1..=0xFE).contains(code)).ok_or(OutputError::Unsupported)?;
        self.link.send_key(KeyCode(key), down).map_err(OutputError::from_link)
    }

    fn send_pointer(&self, op: PointerOp) -> Result<(), OutputError> {
        self.link.send_pointer(op).map_err(OutputError::from_link)
    }

    fn send_move(&self, intent: MoveIntent) -> Result<(), OutputError> {
        self.link.send_move(intent).map_err(OutputError::from_link)
    }

    // the link alone has no client: `GamePort` selects through the game HUD
    fn select_hotbar(&self, _index: u8) -> Result<(), OutputError> {
        Err(OutputError::Unsupported)
    }

    fn panel_open(&self) -> bool {
        self.link.panel_open()
    }
}

impl ScreenshotPort for HeadlessLink {
    fn supports_screenshots(&self) -> bool {
        self.screenshots
    }

    fn request_screenshot(&self, path: &Path, layer: ScreenshotLayer) -> Result<u32, OutputError> {
        let layers = match layer {
            ScreenshotLayer::Game => ScreenshotLayers::GameOnly,
            ScreenshotLayer::Mods => ScreenshotLayers::WithModLayers,
        };
        let id = self.link.request_screenshot_with(path, layers).map_err(OutputError::from_link)?;
        Ok(ScreenshotDesk::sequence(id))
    }
}
