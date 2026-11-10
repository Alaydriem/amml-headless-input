use headless_input_core::player::PlayerView;

// What one frame reads of the game: the player view and whether the player is in a world. `fresh` is true when the position, angles and dimension were read on this frame; otherwise
// the position and angles are zero and the dimension is unknown (no local player, or a read met a null pointer).
// `in_world` is true whenever `view.screen` is InWorld
#[derive(Debug, Clone, PartialEq)]
pub struct FrameView {
    pub view: PlayerView,
    pub in_world: bool,
    pub fresh: bool,
}
