use serde::Deserialize;

// The `mcp` block of the mod's `config.hcl`, as far as the pair client reads it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RawMcp {
    pub port: Option<i64>,
}
