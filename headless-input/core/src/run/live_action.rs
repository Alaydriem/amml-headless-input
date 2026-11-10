use std::net::Ipv4Addr;

use crate::control::{PointerAction, Ticks};
use crate::player::KeySet;

// One live call. Each acts like the step of the same name; actions of different kinds run at the same time.
#[derive(Debug, Clone, PartialEq)]
pub enum LiveAction {
    // holds the keys for the ticks given, or until the session ends; other held keys stay held
    Hold { keys: KeySet, ticks: Option<Ticks> },
    Turn { yaw: f32, pitch: f32, ticks: Ticks },
    LookAt { yaw: f32, pitch: f32, ticks: Ticks },
    // 0 hold ticks is one click
    Pointer { action: PointerAction, hold_ticks: u32 },
    Hotbar { slot: u8 },
    Scroll { steps: i32 },
    // `host` is the name given; `address` is the IPv4 address it resolved to. No timeout uses the goal timeout of the
    // run defaults
    JoinServer { host: String, address: Ipv4Addr, port: u16, timeout: Option<Ticks> },
    LeaveWorld { timeout: Option<Ticks> },
    // ends the live session
    Release,
}
