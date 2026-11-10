use std::net::Ipv4Addr;

use headless_input_core::control::{InputKey, PointerAction, Ticks};
use headless_input_core::player::KeySet;
use headless_input_core::run::LiveAction;
use serde::{Deserialize, Serialize};

use crate::port::PortError;

// The JSON form of a `LiveAction`, as the tools build it and the member link carries it. Durations are duration text
// ("2s", "40t"), checked on the instance that runs the action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiveActionDto {
    Hold {
        keys: Vec<String>,
        #[serde(rename = "for")]
        for_time: Option<String>,
    },
    Turn {
        yaw: f32,
        pitch: f32,
        over: String,
    },
    LookAt {
        yaw: f32,
        pitch: f32,
        over: String,
    },
    Pointer {
        action: String,
        hold: Option<String>,
    },
    Hotbar {
        slot: i64,
    },
    Scroll {
        steps: i32,
    },
    JoinServer {
        address: String,
        port: u16,
        timeout: Option<String>,
    },
    LeaveWorld {
        timeout: Option<String>,
    },
    Release,
}

impl LiveActionDto {
    // the action, or a `Bad` error naming the value that is wrong
    // `resolve` gives the IPv4 address of a join_server host, or None when it has none
    pub fn action(&self, resolve: impl Fn(&str) -> Option<Ipv4Addr>) -> Result<LiveAction, PortError> {
        Ok(match self {
            Self::Hold { keys, for_time } => {
                if keys.is_empty() {
                    return Err(PortError::Bad("hold names no key".to_owned()));
                }
                let mut set = KeySet::new();
                for name in keys {
                    let key =
                        InputKey::from_name(name).ok_or_else(|| PortError::Bad(format!("unknown key \"{name}\"")))?;
                    set.insert(key);
                }
                LiveAction::Hold { keys: set, ticks: Self::optional(for_time.as_deref(), "for")? }
            }
            Self::Turn { yaw, pitch, over } => {
                LiveAction::Turn { yaw: *yaw, pitch: *pitch, ticks: Self::ticks(over, "over")? }
            }
            Self::LookAt { yaw, pitch, over } => {
                LiveAction::LookAt { yaw: *yaw, pitch: *pitch, ticks: Self::ticks(over, "over")? }
            }
            Self::Pointer { action, hold } => {
                let action = match action.as_str() {
                    "attack" => PointerAction::Attack,
                    "use" => PointerAction::Use,
                    other => return Err(PortError::Bad(format!("unknown pointer action \"{other}\""))),
                };
                let hold_ticks = Self::optional(hold.as_deref(), "hold")?.map_or(0, Ticks::get);
                LiveAction::Pointer { action, hold_ticks }
            }
            Self::Hotbar { slot } => match u8::try_from(*slot) {
                Ok(slot @ 1..=9) => LiveAction::Hotbar { slot },
                _ => return Err(PortError::Bad("slot must be from 1 to 9".to_owned())),
            },
            Self::Scroll { steps } => LiveAction::Scroll { steps: *steps },
            Self::JoinServer { address, port, timeout } => {
                if address.trim().is_empty() {
                    return Err(PortError::Bad("address is empty".to_owned()));
                }
                let resolved = resolve(address.trim()).ok_or_else(|| {
                    PortError::Bad(format!("address \"{address}\" did not resolve to an IPv4 address"))
                })?;
                LiveAction::JoinServer {
                    host: address.trim().to_owned(),
                    address: resolved,
                    port: *port,
                    timeout: Self::optional(timeout.as_deref(), "timeout")?,
                }
            }
            Self::LeaveWorld { timeout } => {
                LiveAction::LeaveWorld { timeout: Self::optional(timeout.as_deref(), "timeout")? }
            }
            Self::Release => LiveAction::Release,
        })
    }

    fn optional(text: Option<&str>, field: &str) -> Result<Option<Ticks>, PortError> {
        text.map(|text| Self::ticks(text, field)).transpose()
    }

    fn ticks(text: &str, field: &str) -> Result<Ticks, PortError> {
        Ticks::parse(text).ok_or_else(|| PortError::Bad(format!("{field} \"{text}\" is not a duration")))
    }
}
