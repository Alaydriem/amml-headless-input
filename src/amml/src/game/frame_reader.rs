use amml_api::{Client, LocalPlayer};
use headless_input_core::player::{PlayerView, ScreenKind};
use headless_input_core::structs::Capabilities;

use crate::game::ScreenReader;
use crate::types::FrameView;

// Reads the local player and the top screen through the loader. A read whose capability is off leaves its field at
// zero (the dimension unknown); a read whose capability is on and that the loader refuses gives no view. The position is
// the player's eyes, the game's own position of the local player. Game thread
pub struct FrameReader {
    capabilities: Capabilities,
}

impl FrameReader {
    pub fn new(capabilities: Capabilities) -> Self {
        Self { capabilities }
    }

    // the view of the local player on this tick; None without a local player or when a read that is on is refused
    pub fn read(&self, client: &Client<'_>) -> Option<PlayerView> {
        let player = client.local_player()?;
        let mut view = Self::empty(ScreenKind::InWorld);
        self.fill(&player, &mut view)?;
        Some(view)
    }

    // what this frame shows: the top screen and, when there is a local player, its view. A toast on top gives
    // `previous`, the screen of the last frame
    pub fn frame(&self, client: &Client<'_>, previous: &ScreenKind) -> FrameView {
        let player = client.local_player();
        let name = client.top_screen().map(ScreenReader::name_of).unwrap_or("");
        let screen = ScreenReader::kind(name, previous, player.is_some());
        let in_world = screen == ScreenKind::InWorld || player.is_some();
        let mut view = Self::empty(screen);
        let mut fresh = false;
        if let Some(player) = &player {
            fresh = self.fill(player, &mut view).is_some();
            if !fresh {
                view = Self::empty(view.screen);
            }
        }
        FrameView { view, in_world, fresh }
    }

    fn empty(screen: ScreenKind) -> PlayerView {
        PlayerView { position: [0.0; 3], yaw: 0.0, pitch: 0.0, dimension: None, screen }
    }

    // the position, angles and dimension whose capabilities are on; None when the loader refuses one of them
    fn fill(&self, player: &LocalPlayer<'_>, view: &mut PlayerView) -> Option<()> {
        if self.capabilities.position {
            let eye = player.eye_position()?;
            view.position = [eye.x, eye.y, eye.z];
        }
        if self.capabilities.rotation {
            let rotation = player.rotation()?;
            view.pitch = rotation.pitch;
            view.yaw = rotation.yaw;
        }
        if self.capabilities.dimension {
            view.dimension = Some(player.dimension()?.id()?);
        }
        Some(())
    }
}
