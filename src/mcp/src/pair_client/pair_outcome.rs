/// What a pair request to the headless-input hub gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairOutcome {
    /// The person clicked Allow: the new token.
    Token(String),
    /// The person clicked Deny, or did not answer in time (403).
    Denied,
    /// Another pairing dialog is open (409).
    Busy,
    /// Nothing listens on the port: the MCP server is off.
    NotRunning,
    /// Any other failure, with what went wrong.
    Failed(String),
}
