use super::ScreenKind;

// What the game shows of the player on one tick.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerView {
    pub position: [f32; 3],
    // Minecraft yaw in degrees: 0 faces +Z, 90 faces -X
    pub yaw: f32,
    // degrees, positive looks down
    pub pitch: f32,
    // the game dimension id; None while it is not known (no local player, or the read failed)
    pub dimension: Option<i32>,
    pub screen: ScreenKind,
}
