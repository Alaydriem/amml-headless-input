use serde::Deserialize;

// The attributes of the defaults block.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawDefaults {
    pub goal_timeout: Option<String>,
}
