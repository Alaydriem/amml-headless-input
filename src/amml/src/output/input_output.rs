use amm_client::link::PointerOp as ManagerPointerOp;
use amml_api::{Look, MoveIntent};
use headless_input_core::control::{InputKey, PointerAction, TapKey};
use headless_input_core::player::{InputFrame, KeySet, PointerOp};

use crate::errors::OutputError;
use crate::output::{KeyCodes, LeaseHolder, MoveMode};
use crate::traits::ManagerPort;

// Sends an `InputFrame` to the manager as changes: the movement keys are sent as one move command when they change
// (as fake keys to a loader without the move area), a button goes down or up only when the frame differs from what the
// mod holds, a hotbar slot is selected through the game HUD, a pressed key is tapped (down on its frame, up on the
// next), a scroll turns the wheel, and a rotation turns the look. The record holds only what the manager took, so `release_all` sends an up
// for each thing still down. The look is turned through the manager as Turn actions: the output keeps the look it
// commanded last and sends the difference to the wanted look. Nothing allocates
pub struct InputOutput {
    keys: KeySet,
    // how `keys` reached the manager; it resets with the record, so each run asks the manager once
    mode: MoveMode,
    attack: bool,
    use_item: bool,
    // the pressed key's code from the last frame, let go on this one
    tap: Option<u32>,
    // the (yaw, pitch) the last turn commanded, in degrees; None before the first rotation of a step
    look: Option<(f32, f32)>,
}

impl InputOutput {
    // the highest pitch either way: straight down or straight up
    pub const MAX_PITCH: f32 = 90.0;

    pub fn new() -> Self {
        Self { keys: KeySet::new(), mode: MoveMode::Unknown, attack: false, use_item: false, tap: None, look: None }
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
        self.send_tap(frame.tap, port)?;
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

    // the move cleared (or an up for every held movement key), then an up for the pressed key and
    // every button; a refused up is dropped, so the record ends empty
    pub fn release_all(&mut self, port: &impl ManagerPort) {
        if !self.keys.is_empty() {
            match self.mode {
                MoveMode::Command => {
                    let _ = port.send_move(MoveIntent::default());
                }
                MoveMode::Keys | MoveMode::Unknown => {
                    for key in InputKey::ALL {
                        if self.keys.contains(key) {
                            let _ = port.send_key(KeyCodes::key(key), false);
                        }
                    }
                }
            }
        }
        if let Some(vk) = self.tap {
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
    // short way, the pitch held to straight up and down, as one turn in degrees. The game's angles and the turn use the
    // same signs (positive yaw turns right seen from above, positive pitch looks down). A frame with no rotation
    // forgets the commanded look
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
        let Some((at_yaw, at_pitch)) = self.look.or(read) else {
            return Ok(());
        };
        self.look = Some((at_yaw, at_pitch));
        let wanted_pitch = pitch.clamp(-Self::MAX_PITCH, Self::MAX_PITCH);
        let turn = Look { yaw: Self::wrap_degrees(yaw - at_yaw), pitch: wanted_pitch - at_pitch };
        if turn.yaw == 0.0 && turn.pitch == 0.0 {
            return Ok(());
        }
        port.send_pointer(ManagerPointerOp::Turn(turn))?;
        self.look = Some((Self::wrap_degrees(at_yaw + turn.yaw), at_pitch + turn.pitch));
        Ok(())
    }

    // the movement keys as one move command; a manager without it gets them as fake keys for the rest of the run
    fn send_keys(&mut self, wanted: KeySet, port: &impl ManagerPort) -> Result<(), OutputError> {
        if wanted == self.keys {
            return Ok(());
        }
        if self.mode != MoveMode::Keys {
            match port.send_move(Self::intent(wanted)) {
                Ok(()) => {
                    self.mode = MoveMode::Command;
                    self.keys = wanted;
                    return Ok(());
                }
                Err(OutputError::Unsupported) => self.mode = MoveMode::Keys,
                Err(error) => return Err(error),
            }
        }
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

    // forward is W less S, left is A less D; Space, Shift and Ctrl are jump, sneak and sprint
    fn intent(keys: KeySet) -> MoveIntent {
        let axis = |plus: InputKey, minus: InputKey| {
            f32::from(u8::from(keys.contains(plus))) - f32::from(u8::from(keys.contains(minus)))
        };
        MoveIntent {
            forward: axis(InputKey::Forward, InputKey::Back),
            left: axis(InputKey::Left, InputKey::Right),
            jump: keys.contains(InputKey::Jump),
            sneak: keys.contains(InputKey::Sneak),
            sprint: keys.contains(InputKey::Sprint),
        }
    }

    // slot 1 to 9 of the frame is HUD slot 0 to 8; any other slot is dropped
    fn send_hotbar(&self, slot: Option<u8>, port: &impl ManagerPort) -> Result<(), OutputError> {
        match slot.and_then(KeyCodes::hotbar_index) {
            Some(index) => port.select_hotbar(index),
            None => Ok(()),
        }
    }

    fn send_tap(&mut self, tap: Option<TapKey>, port: &impl ManagerPort) -> Result<(), OutputError> {
        if let Some(vk) = self.tap {
            port.send_key(vk, false)?;
            self.tap = None;
        }
        if let Some(key) = tap {
            port.send_key(key.vk(), true)?;
            self.tap = Some(key.vk());
            curia::info!("key tap sent", { key: format!("{:#x}", key.vk()) });
        }
        Ok(())
    }

    fn send_pointer(&mut self, op: PointerOp, port: &impl ManagerPort) -> Result<(), OutputError> {
        match op {
            PointerOp::Press(action) => self.send_button(action, true, port),
            PointerOp::Release(action) => self.send_button(action, false, port),
            PointerOp::Scroll(0) => Ok(()),
            PointerOp::Scroll(steps) => port.send_pointer(ManagerPointerOp::Wheel { notches: steps as f32 }),
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
