use serde::Deserialize;

// The attributes of a repeat block.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRepeat {
    pub count: Option<i64>,
    pub timeout: Option<String>,
}
