use serde::Deserialize;

use super::RawMcp;

// The part of the mod's `config.hcl` the pair client reads; every other block is ignored.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RawConfig {
    pub mcp: RawMcp,
}
