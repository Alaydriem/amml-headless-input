use std::path::Path;

use amm_client::link::{ApiTable, EventDrain, ManagerLink};
use amm_core::errors::LinkError;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amm_core::structs::input::PointerOp;
use amm_core::structs::lease::AcquireOutcome;
use headless_input_core::config::HeadlessConfig;

use crate::errors::{LeaseError, OutputError};
use crate::services::{HeadlessDescriptor, RawSettings, SettingsReader};
use crate::traits::ManagerPort;

// the registration with the mod manager and the settings read from it; `events` holds at most
// MAX_EVENTS_PER_TICK and is cleared at the start of every drain
pub struct HeadlessLink {
    link: ManagerLink,
    config: HeadlessConfig,
    events: Vec<ModEvent>,
}

impl HeadlessLink {
    pub const MAX_DRAINS_PER_TICK: usize = 4;
    pub const MAX_EVENTS_PER_TICK: usize = Self::MAX_DRAINS_PER_TICK * EventDrain::CAPACITY;
    // the holder named when the manager denies the lease and does not say who holds it
    // the setting that allows the MCP pairing request that waits
    pub const PAIRING_SETTING: &'static str = "mcp.allow_pairing";
    pub const UNKNOWN_HOLDER: &'static str = "another mod";

    pub fn new(table: ApiTable, config_dir: &Path) -> Result<Self, LinkError> {
        let link = ManagerLink::without_frame(table, HeadlessDescriptor::TEXT, config_dir)?;
        Self::from_link(link)
    }

    pub fn from_link(link: ManagerLink) -> Result<Self, LinkError> {
        let mut this =
            Self { config: HeadlessConfig::default(), link, events: Vec::with_capacity(Self::MAX_EVENTS_PER_TICK) };
        this.config = this.read_settings();
        Ok(this)
    }

    pub fn config(&self) -> &HeadlessConfig {
        &self.config
    }

    // reads the settings again
    pub fn reload(&mut self) -> &HeadlessConfig {
        self.config = self.read_settings();
        &self.config
    }

    // reads the queued manager events into the reused buffer, at most MAX_EVENTS_PER_TICK; the rest wait for the next
    // tick. The slice is valid until the next drain
    pub fn drain(&mut self) -> &[ModEvent] {
        self.events.clear();
        for _ in 0..Self::MAX_DRAINS_PER_TICK {
            let before = self.events.len();
            self.events.extend(self.link.events());
            if self.events.len() - before < EventDrain::CAPACITY {
                break;
            }
        }
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

    fn send_key(&self, vk: u32, down: bool) -> Result<(), OutputError> {
        self.link.send_key(vk, down).map_err(OutputError::from_link)
    }

    fn send_pointer(&self, op: PointerOp) -> Result<(), OutputError> {
        self.link.send_pointer(op).map_err(OutputError::from_link)
    }

    fn panel_open(&self) -> bool {
        self.link.panel_open()
    }
}
