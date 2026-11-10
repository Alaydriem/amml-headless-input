// one pairing question and its answer: the mcp.allow_pairing setting turning on while it waits allows it
#[derive(Debug, Clone, PartialEq)]
pub struct PromptState {
    pub asking: bool,
    pub answer: Option<bool>,
    // the mod is disabled: every question is denied at once
    pub closed: bool,
}

impl PromptState {
    pub fn new() -> Self {
        Self { asking: false, answer: None, closed: false }
    }
}

impl Default for PromptState {
    fn default() -> Self {
        Self::new()
    }
}
