use crate::port::PortError;

// The instance a tool names: one by id, every one (`"*"`), or none given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceTarget {
    One(String),
    All,
    Default,
}

impl InstanceTarget {
    pub const ALL: &'static str = "*";

    pub fn new(instance: Option<&str>) -> Self {
        match instance.map(str::trim) {
            None | Some("") => Self::Default,
            Some(Self::ALL) => Self::All,
            Some(name) => Self::One(name.to_owned()),
        }
    }

    // the ids the tool runs on, out of `connected`. With none given, one connected instance is the target and more
    // is an error that lists them. `"*"` is only for the tools that take it
    pub fn resolve(&self, connected: &[String], allow_all: bool) -> Result<Vec<String>, PortError> {
        match self {
            Self::One(name) if connected.iter().any(|id| id == name) => Ok(vec![name.clone()]),
            Self::One(name) => Err(PortError::UnknownInstance(name.clone())),
            Self::All if allow_all => Ok(connected.to_vec()),
            Self::All => Err(PortError::Bad("\"*\" is only for run_control_file, stop and release".to_owned())),
            Self::Default => match connected {
                [only] => Ok(vec![only.clone()]),
                [] => Err(PortError::UnknownInstance("(none connected)".to_owned())),
                _ => Err(PortError::NeedsInstance { connected: connected.to_vec() }),
            },
        }
    }
}
