// The settings of the mod; durations are held in ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessConfig {
    // serves MCP from this instance
    pub mcp_enabled: bool,
    // the hub port
    pub mcp_port: u16,
    // the switch that allows the MCP pairing request that waits; the mod turns it off after each answer
    pub mcp_allow_pairing: bool,
    // the longest a goal step may run before the run fails
    pub goal_timeout_ticks: u32,
    // the longest a run may go without progress
    pub watchdog_ticks: u32,
    // a live session ends after this long without a command
    pub live_idle_ticks: u32,
    // the folder of control files; empty is the mod data folder
    pub files_folder: String,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            mcp_enabled: false,
            mcp_port: 28250,
            mcp_allow_pairing: false,
            goal_timeout_ticks: 600,
            watchdog_ticks: 100,
            live_idle_ticks: 600,
            files_folder: String::new(),
        }
    }
}
