/// What a `claude mcp add` run gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeOutcome {
    /// `claude` added the `headless-input` server.
    Added,
    /// `claude` is not on the path; the line is the command to run by hand.
    NotFound(String),
    /// `claude` ran and failed; the text is what it wrote.
    Failed(String),
}
