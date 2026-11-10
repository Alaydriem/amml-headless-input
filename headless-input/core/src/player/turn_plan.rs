// A rotation spread evenly over a number of ticks. Yaw is Minecraft yaw (0 faces +Z, 90 faces -X), kept in
// [-180, 180); pitch is clamped to [-90, 90].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnPlan {
    from: (f32, f32),
    delta: (f32, f32),
    to: (f32, f32),
    ticks: u32,
}

impl TurnPlan {
    pub const PITCH_LIMIT: f32 = 90.0;

    // a turn by (yaw, pitch) from the rotation `from`; the yaw turns the way its sign says, a full turn included
    pub fn by(from: (f32, f32), turn: (f32, f32), ticks: u32) -> Self {
        let to = (Self::wrap(from.0 + turn.0), Self::clamp_pitch(from.1 + turn.1));
        Self { from, delta: (turn.0, to.1 - from.1), to, ticks: ticks.max(1) }
    }

    // a turn from the rotation `from` to the rotation `target`, the short way round
    pub fn toward(from: (f32, f32), target: (f32, f32), ticks: u32) -> Self {
        let to = (Self::wrap(target.0), Self::clamp_pitch(target.1));
        Self { from, delta: (Self::wrap(to.0 - from.0), to.1 - from.1), to, ticks: ticks.max(1) }
    }

    // the rotation at the end of tick `tick` of the plan, counted from 0; the target from the last tick on
    pub fn at(&self, tick: u32) -> (f32, f32) {
        let done = tick.saturating_add(1);
        if done >= self.ticks {
            return self.to;
        }
        let part = done as f32 / self.ticks as f32;
        (Self::wrap(self.from.0 + self.delta.0 * part), self.from.1 + self.delta.1 * part)
    }

    pub fn target(&self) -> (f32, f32) {
        self.to
    }

    // the yaw that faces (x, z) from `position`
    pub fn bearing(position: [f32; 3], x: f32, z: f32) -> f32 {
        let dx = x - position[0];
        let dz = z - position[2];
        Self::wrap((-dx).atan2(dz).to_degrees())
    }

    // an angle in degrees brought into [-180, 180)
    pub fn wrap(angle: f32) -> f32 {
        (angle + 180.0).rem_euclid(360.0) - 180.0
    }

    fn clamp_pitch(pitch: f32) -> f32 {
        pitch.clamp(-Self::PITCH_LIMIT, Self::PITCH_LIMIT)
    }
}
