use headless_input_core::config::{DurationText, HeadlessConfig};
use serde::Deserialize;

// the part of config.hcl the mod reads; a missing value keeps its default
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RawSettings {
    mcp: RawMcp,
    run: RawRun,
    live: RawLive,
    files: RawFiles,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawMcp {
    enabled: Option<bool>,
    port: Option<i64>,
    allow_pairing: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawRun {
    goal_timeout: Option<String>,
    watchdog: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawLive {
    idle: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawFiles {
    folder: Option<String>,
}

// turns the checked config.hcl into a HeadlessConfig
pub struct SettingsReader;

impl SettingsReader {
    // text that cannot be read gives the defaults, with one warning
    pub fn read(text: &str) -> HeadlessConfig {
        match hcl::from_str::<RawSettings>(text) {
            Ok(raw) => Self::convert(&raw),
            Err(error) => {
                curia::warn!("config could not be read; using the defaults", { error: error.to_string() });
                HeadlessConfig::default()
            }
        }
    }

    // a value that is out of range or not a duration keeps its default
    pub fn convert(raw: &RawSettings) -> HeadlessConfig {
        let defaults = HeadlessConfig::default();
        HeadlessConfig {
            mcp_enabled: raw.mcp.enabled.unwrap_or(defaults.mcp_enabled),
            mcp_port: raw.mcp.port.and_then(|port| u16::try_from(port).ok()).unwrap_or(defaults.mcp_port),
            mcp_allow_pairing: raw.mcp.allow_pairing.unwrap_or(defaults.mcp_allow_pairing),
            goal_timeout_ticks: Self::ticks(raw.run.goal_timeout.as_deref(), defaults.goal_timeout_ticks),
            watchdog_ticks: Self::ticks(raw.run.watchdog.as_deref(), defaults.watchdog_ticks),
            live_idle_ticks: Self::ticks(raw.live.idle.as_deref(), defaults.live_idle_ticks),
            files_folder: raw.files.folder.clone().unwrap_or(defaults.files_folder),
        }
    }

    fn ticks(text: Option<&str>, default: u32) -> u32 {
        text.and_then(DurationText::parse_ticks).unwrap_or(default)
    }
}
