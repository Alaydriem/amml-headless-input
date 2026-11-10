use alaydriem_headless_input::services::SettingsReader;
use headless_input_core::config::HeadlessConfig;

#[test]
fn unreadable_settings_give_the_defaults() {
    assert_eq!(SettingsReader::read("run = {"), HeadlessConfig::default());
}

#[test]
fn a_bad_duration_keeps_the_default() {
    let config = SettingsReader::read("run {\n  watchdog = \"soon\"\n}\n");
    assert_eq!(config.watchdog_ticks, HeadlessConfig::default().watchdog_ticks);
}

#[test]
fn every_value_is_read() {
    let text = "mcp {\n  enabled = true\n  port = 29000\n  allow_pairing = true\n}\n\
                run {\n  goal_timeout = \"10s\"\n  watchdog = \"0.5s\"\n}\n\
                live {\n  idle = \"2s\"\n}\n\
                files {\n  folder = \"C:/runs\"\n}\n";
    let config = SettingsReader::read(text);
    assert!(config.mcp_enabled);
    assert_eq!(config.mcp_port, 29000);
    assert!(config.mcp_allow_pairing);
    assert!(!HeadlessConfig::default().mcp_allow_pairing);
    assert_eq!(config.goal_timeout_ticks, 200);
    assert_eq!(config.watchdog_ticks, 10);
    assert_eq!(config.live_idle_ticks, 40);
    assert_eq!(config.files_folder, "C:/runs");
}

#[test]
fn a_port_outside_the_range_keeps_the_default() {
    let config = SettingsReader::read("mcp {\n  port = 70000\n}\n");
    assert_eq!(config.mcp_port, HeadlessConfig::default().mcp_port);
}
