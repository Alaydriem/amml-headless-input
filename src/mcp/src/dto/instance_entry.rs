use serde::{Deserialize, Serialize};

use super::FeaturesDto;

// One line of the `instances` tool: a connected instance, whether it is the hub, its state and its features.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceEntry {
    pub instance: String,
    pub hub: bool,
    pub pid: u32,
    // idle, running or live; unknown when the instance did not answer
    pub state: String,
    pub features: FeaturesDto,
}
