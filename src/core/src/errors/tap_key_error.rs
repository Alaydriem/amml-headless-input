// A key name that `press` cannot tap.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TapKeyError {
    #[error("unknown key \"{0}\" (use a key name such as \"E\", \"Esc\" or \"F5\")")]
    Unknown(String),
    #[error("key \"{name}\" is driven by the {tool} tool")]
    Owned { name: String, tool: &'static str },
}
