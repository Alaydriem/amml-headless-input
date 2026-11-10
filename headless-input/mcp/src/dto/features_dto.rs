use headless_input_core::structs::Capabilities;
use serde::{Deserialize, Serialize};

// The JSON form of `Capabilities`: the reads and game calls the game build of one instance gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturesDto {
    pub position: bool,
    pub rotation: bool,
    pub dimension: bool,
    pub join: bool,
    pub leave: bool,
    // an older member that does not report it cannot send a command
    #[serde(default)]
    pub command: bool,
}

impl FeaturesDto {
    pub fn capabilities(&self) -> Capabilities {
        Capabilities::new(self.position, self.rotation, self.dimension, self.join, self.leave, self.command)
    }
}

impl From<Capabilities> for FeaturesDto {
    fn from(capabilities: Capabilities) -> Self {
        Self {
            position: capabilities.position,
            rotation: capabilities.rotation,
            dimension: capabilities.dimension,
            join: capabilities.join,
            leave: capabilities.leave,
            command: capabilities.command,
        }
    }
}
