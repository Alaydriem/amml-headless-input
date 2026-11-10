use super::Step;

// A checked control file.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlFile {
    pub name: String,
    pub steps: Vec<Step>,
    // None when a goal step or a repeat with no count leaves the length open; the wall-clock cap then uses the sum of
    // all timeouts
    pub total_ticks: Option<u32>,
}

impl ControlFile {
    pub const MAX_DEPTH: usize = 4;
    // after counted repeats are expanded
    pub const MAX_STEPS: u64 = 512;
    // one hour
    pub const MAX_TICKS: u64 = 72_000;
    pub const MAX_BYTES: usize = 65_536;
}
