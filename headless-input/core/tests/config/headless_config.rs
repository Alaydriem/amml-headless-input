use headless_input_core::config::HeadlessConfig;

#[test]
fn defaults_hold_the_documented_values() {
    let config = HeadlessConfig::default();
    assert!(!config.mcp_enabled);
    assert_eq!(config.mcp_port, 28250);
    assert_eq!(config.goal_timeout_ticks, 600);
    assert_eq!(config.watchdog_ticks, 100);
    assert_eq!(config.live_idle_ticks, 600);
    assert_eq!(config.files_folder, "");
}
