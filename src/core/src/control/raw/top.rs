use serde::Deserialize;

// The attributes at the top of a control file.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawTop {
    pub name: Option<String>,
}
