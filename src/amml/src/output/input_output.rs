use amm_core::structs::input::{PointerOp as ManagerPointerOp, TurnAxis};
use headless_input_core::control::{InputKey, PointerAction};
use headless_input_core::player::{InputFrame, KeySet, PointerOp};

use crate::errors::OutputError;
use crate::output::{KeyCodes, LeaseHolder};
use crate::traits::ManagerPort;

// Sends an `InputFrame` to the manager as changes: a key or button goes down or up only when the frame differs from
// what the mod holds, a hotbar slot is tapped (down on its frame, up on the next), a scroll turns the wheel, and a
// rotation turns the look. The record holds only what the manager took, so `release_all` sends an up for each thing
// still down. The look is turned through the manager as Turn actions: the output keeps the look it commanded last and
// sends the difference to the wanted look. Nothing allocates
pub struct InputOutput {
    keys: KeySet,
    attack: bool,
    use_item: bool,
    // the hotbar key pressed on the last frame, let go on this one
    hotbar: Option<u32>,
    // the (yaw, pitch) the last turn commanded, in degrees; None before the first rotation of a step
    look: Option<(f32, f32)>,
}

impl InputOutput {
    // the most wheel notches the manager takes in one action; a longer scroll is sent in turns
    pub const MAX_WHEEL_STEPS: i32 = 32;
    // the highest pitch either way: straight down or straight up
    pub const MAX_PITCH: f32 = 90.0;
    const MILLIDEGREES: f32 = 1000.0;

    pub fn new() -> Self {
        Self { keys: KeySet::new(), attack: false, use_item: false, hotbar: None, look: None }
    }

    // the keys, the hotbar tap and the pointer events of one frame, then its rotation. On the first send the manager
    // refuses, the rest of the frame is not sent and the record keeps what the manager took, so `release_all` still
    // sends their ups: a refusal for another mod's lease leaves the held keys down in the manager. The rotation is
    // sent only after every other send was taken and while the manager panel is closed. `read` is the (yaw, pitch)
    // the game reports now; the first rotation of a step turns from it, and none is sent while it is None and no
    // look was commanded yet
    pub fn apply(
        &mut self,
        frame: &InputFrame,
        port: &impl ManagerPort,
        read: Option<(f32, f32)>,
    ) -> Result<(), OutputError> {
        self.send_keys(frame.keys, port)?;
        self.send_hotbar(frame.hotbar, port)?;
        for op in frame.pointer.as_slice() {
            self.send_pointer(*op, port)?;
        }
        self.send_rotation(frame.rotation, read, port)
    }

    // an angle in degrees wrapped to -180..180
    pub fn wrap_degrees(angle: f32) -> f32 {
        let mut wrapped = (angle + 180.0) % 360.0;
        if wrapped < 0.0 {
            wrapped += 360.0;
        }
        wrapped - 180.0
    }

    // an up for every held key, hotbar key and button; a refused up is dropped, so the record ends empty
    pub fn release_all(&mut self, port: &impl ManagerPort) {
        for key in InputKey::ALL {
            if self.keys.contains(key) {
                let _ = port.send_key(KeyCodes::key(key), false);
            }
        }
        if let Some(vk) = self.hotbar {
            let _ = port.send_key(vk, false);
        }
        for action in [PointerAction::Attack, PointerAction::Use] {
            if self.held(action) {
                let _ = port.send_pointer(ManagerPointerOp::Button { button: KeyCodes::button(action), down: false });
            }
        }
        self.forget();
    }

    // every run end: the held input is released, then the lease. The order is fixed
    pub fn end(&mut self, port: &impl ManagerPort, lease: &mut LeaseHolder) {
        self.release_all(port);
        lease.end(port);
    }

    // drops the record without sending, only for `FakeKeysReleased`: the manager let go of the mod's fake input itself
    pub fn forget(&mut self) {
        *self = Self::new();
    }

    // the turn from the last commanded look (the read one at the first rotation) to the wanted look: the yaw by the
    // short way, the pitch held to straight up and down. A frame with no rotation forgets the commanded look. The
    // commanded look moves by what the manager took, in millidegrees, so a rounding step is sent on a later frame
    fn send_rotation(
        &mut self,
        rotation: Option<(f32, f32)>,
        read: Option<(f32, f32)>,
        port: &impl ManagerPort,
    ) -> Result<(), OutputError> {
        let Some((yaw, pitch)) = rotation else {
            self.look = None;
            return Ok(());
        };
        if !yaw.is_finite() || !pitch.is_finite() || port.panel_open() {
            return Ok(());
        }
        let Some((mut at_yaw, mut at_pitch)) = self.look.or(read) else {
            return Ok(());
        };
        self.look = Some((at_yaw, at_pitch));
        let wanted_pitch = pitch.clamp(-Self::MAX_PITCH, Self::MAX_PITCH);
        let yaw_step = (Self::wrap_degrees(yaw - at_yaw) * Self::MILLIDEGREES).round() as i32;
        let pitch_step = ((wanted_pitch - at_pitch) * Self::MILLIDEGREES).round() as i32;
        if yaw_step != 0 {
            port.send_pointer(ManagerPointerOp::Turn { axis: TurnAxis::Yaw, millidegrees: yaw_step })?;
            at_yaw = Self::wrap_degrees(at_yaw + yaw_step as f32 / Self::MILLIDEGREES);
            self.look = Some((at_yaw, at_pitch));
        }
        if pitch_step != 0 {
            // the camera turns the pitch the other way from `mRot`: a positive turn delta lowers the read pitch
            port.send_pointer(ManagerPointerOp::Turn { axis: TurnAxis::Pitch, millidegrees: -pitch_step })?;
            at_pitch += pitch_step as f32 / Self::MILLIDEGREES;
            self.look = Some((at_yaw, at_pitch));
        }
        Ok(())
    }

    fn send_keys(&mut self, wanted: KeySet, port: &impl ManagerPort) -> Result<(), OutputError> {
        for key in InputKey::ALL {
            let down = wanted.contains(key);
            if down != self.keys.contains(key) {
                port.send_key(KeyCodes::key(key), down)?;
                if down {
                    self.keys.insert(key);
                } else {
                    self.keys.remove(key);
                }
            }
        }
        Ok(())
    }

    fn send_hotbar(&mut self, slot: Option<u8>, port: &impl ManagerPort) -> Result<(), OutputError> {
        if let Some(vk) = self.hotbar {
            port.send_key(vk, false)?;
            self.hotbar = None;
        }
        if let Some(vk) = slot.and_then(KeyCodes::hotbar) {
            port.send_key(vk, true)?;
            self.hotbar = Some(vk);
        }
        Ok(())
    }

    fn send_pointer(&mut self, op: PointerOp, port: &impl ManagerPort) -> Result<(), OutputError> {
        match op {
            PointerOp::Press(action) => self.send_button(action, true, port),
            PointerOp::Release(action) => self.send_button(action, false, port),
            PointerOp::Scroll(steps) => {
                let mut left = steps;
                while left != 0 {
                    let turn = left.clamp(-Self::MAX_WHEEL_STEPS, Self::MAX_WHEEL_STEPS);
                    port.send_pointer(ManagerPointerOp::Wheel { steps: turn })?;
                    left -= turn;
                }
                Ok(())
            }
        }
    }

    fn send_button(&mut self, action: PointerAction, down: bool, port: &impl ManagerPort) -> Result<(), OutputError> {
        if self.held(action) == down {
            return Ok(());
        }
        port.send_pointer(ManagerPointerOp::Button { button: KeyCodes::button(action), down })?;
        *self.button_mut(action) = down;
        Ok(())
    }

    fn held(&self, action: PointerAction) -> bool {
        match action {
            PointerAction::Attack => self.attack,
            PointerAction::Use => self.use_item,
        }
    }

    fn button_mut(&mut self, action: PointerAction) -> &mut bool {
        match action {
            PointerAction::Attack => &mut self.attack,
            PointerAction::Use => &mut self.use_item,
        }
    }
}

impl Default for InputOutput {
    fn default() -> Self {
        Self::new()
    }
}
