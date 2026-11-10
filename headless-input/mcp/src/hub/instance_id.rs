use std::fmt;

// The id of one game instance: the AMML instance folder name. The tools name instances by it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceId(String);

impl InstanceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    // this id with a process id after it, for a game whose instance folder another running game shares
    pub fn with_pid(&self, pid: u32) -> Self {
        Self(format!("{}-{pid}", self.0))
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
