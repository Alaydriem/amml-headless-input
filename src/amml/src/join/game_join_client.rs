use amml_api::{ApiError, Client, JoinTarget as GameTarget, Menu};
use headless_input_core::structs::{JoinTarget, WorldTarget};

use crate::game::ScreenReader;
use crate::join::{JoinClient, JoinCodes};

// The game's client behind the `JoinClient` trait, through the loader's session actions. A call the loader refuses
// answers no. Game thread
pub struct GameJoinClient<'a, 'f> {
    client: &'a Client<'f>,
    codes: &'a JoinCodes,
}

impl<'a, 'f> GameJoinClient<'a, 'f> {
    pub fn new(client: &'a Client<'f>, codes: &'a JoinCodes) -> Self {
        Self { client, codes }
    }
}

impl JoinClient for GameJoinClient<'_, '_> {
    fn is_primary_ready(&self) -> bool {
        self.client.can_join().unwrap_or(false)
    }

    fn is_in_game(&self) -> bool {
        self.client.local_player().is_some()
    }

    fn top_screen_name(&self) -> String {
        self.client.top_screen().map(ScreenReader::name_of).unwrap_or("").to_string()
    }

    fn request_leave_game(&self) {
        if let Err(error) = self.client.leave_world() {
            curia::warn!("the loader refused the world leave", { error: format!("{error:?}") });
        }
    }

    fn open_servers_tab(&self) -> bool {
        self.client.open_menu(Menu::Servers).is_ok()
    }

    fn join_server(&self, target: &JoinTarget) -> bool {
        let host = target.host.as_str();
        match self.client.join_server(GameTarget { host, port: target.port, name: None }) {
            Ok(_) => true,
            Err(error) => {
                curia::debug!("the loader refused the server join", { error: format!("{error:?}") });
                false
            }
        }
    }

    // the game's "not ready" answer is the core's not ready code; a call the loader refuses otherwise asks again
    fn join_world(&self, target: &WorldTarget) -> Option<i32> {
        match self.client.start_local_world(&target.level_id) {
            Ok(outcome) => Some(JoinCodes::start_code(outcome)),
            Err(ApiError::NotReady) => Some(headless_input_core::run::WorldStartCodes::NOT_READY),
            Err(_) => None,
        }
    }

    fn take_join_result(&self) -> Option<i32> {
        self.codes.take()
    }

    fn clear_join_result(&self) {
        self.codes.clear();
    }
}
