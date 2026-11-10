use serde::Deserialize;

// The hold attribute: a list of keys for a move, a duration for a pointer step.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RawHold {
    One(String),
    Many(Vec<String>),
}

// The attributes of a step block of any kind.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawStep {
    pub hold: Option<RawHold>,
    #[serde(rename = "for")]
    pub for_time: Option<String>,
    pub over: Option<String>,
    pub yaw: Option<f64>,
    pub pitch: Option<f64>,
    pub slot: Option<i64>,
    pub steps: Option<i64>,
    pub text: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub within: Option<f64>,
    pub sprint: Option<bool>,
    pub dimension: Option<String>,
    pub address: Option<String>,
    pub port: Option<i64>,
    pub timeout: Option<String>,
}
