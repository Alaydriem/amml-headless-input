use std::net::Ipv4Addr;
use std::sync::Arc;

use super::{DimensionName, InputKey, PointerAction, TapKey, Ticks};

// One step of a control file.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Move { hold: Vec<InputKey>, ticks: Ticks },
    Jump { ticks: Ticks },
    Sneak { ticks: Ticks },
    Sprint { ticks: Ticks },
    Turn { yaw: f32, pitch: f32, ticks: Ticks },
    LookAt { yaw: f32, pitch: f32, ticks: Ticks },
    // 0 hold ticks is one click
    Pointer { action: PointerAction, hold_ticks: u32 },
    Hotbar { slot: u8 },
    Press { key: TapKey },
    Scroll { steps: i32 },
    // one slash command sent to the server as the local player; the text is shared with every frame that carries it
    Command { text: Arc<str> },
    Wait { ticks: Ticks },
    WalkTo { x: f32, z: f32, y: Option<f32>, within: f32, sprint: bool, timeout: Ticks },
    WalkUntilDimension { dimension: Option<DimensionName>, timeout: Ticks },
    // `host` is the name the file gave; `address` is the IPv4 address it resolved to
    JoinServer { host: String, address: Ipv4Addr, port: u16, timeout: Ticks },
    // `world` is the name the file gave; `level_id` is the folder id it resolved to
    JoinWorld { world: String, level_id: String, timeout: Ticks },
    LeaveWorld { timeout: Ticks },
    // no count repeats until the run stops or the timeout passes
    Repeat { count: Option<u32>, timeout: Option<Ticks>, steps: Vec<Step> },
}

impl Step {
    // true for a step that moves the player or acts on the world, which a screen over the world would block; the other
    // steps may run while a screen is open
    pub fn needs_world(&self) -> bool {
        matches!(
            self,
            Self::Move { .. }
                | Self::Jump { .. }
                | Self::Sneak { .. }
                | Self::Sprint { .. }
                | Self::Turn { .. }
                | Self::LookAt { .. }
                | Self::Pointer { .. }
                | Self::WalkTo { .. }
                | Self::WalkUntilDimension { .. }
        )
    }
}
