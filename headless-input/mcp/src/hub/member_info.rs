use headless_input_core::structs::Capabilities;

use super::InstanceId;

// One instance the hub knows: its id, process, features, and whether it is the hub itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    pub id: InstanceId,
    pub pid: u32,
    pub features: Capabilities,
    // the hub's own instance: its commands skip HTTP
    pub local: bool,
}

impl MemberInfo {
    pub fn new(id: InstanceId, pid: u32, features: Capabilities, local: bool) -> Self {
        Self { id, pid, features, local }
    }
}
