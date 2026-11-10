// Watches whether the movement tick reaches the local player in each world: it notes the first local tick of a
// world, and gives one warning when a world passes FRAMES_BEFORE_WARNING frames with none. A world starts on the
// first in-world frame or local tick after a frame outside a world
pub struct TickWatch {
    in_world: bool,
    matched: bool,
    warned: bool,
    frames: u32,
}

impl TickWatch {
    pub const FRAMES_BEFORE_WARNING: u32 = 200;

    pub fn new() -> Self {
        Self { in_world: false, matched: false, warned: false, frames: 0 }
    }

    // true once per world, on the frame that reaches FRAMES_BEFORE_WARNING with no local tick
    pub fn on_frame(&mut self, in_world: bool) -> bool {
        if !in_world {
            *self = Self::new();
            return false;
        }
        self.in_world = true;
        if self.matched || self.warned {
            return false;
        }
        self.frames = self.frames.saturating_add(1);
        self.warned = self.frames >= Self::FRAMES_BEFORE_WARNING;
        self.warned
    }

    // true for the first local tick of a world
    pub fn on_local_tick(&mut self) -> bool {
        self.in_world = true;
        !std::mem::replace(&mut self.matched, true)
    }
}

impl Default for TickWatch {
    fn default() -> Self {
        Self::new()
    }
}
