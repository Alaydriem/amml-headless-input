use amml_api::Client;

use crate::command::CommandClient;

// The game's client behind the `CommandClient` trait, through the loader's `send_command`. A send the loader refuses
// answers no. Game thread
pub struct GameCommandClient<'a, 'f> {
    client: &'a Client<'f>,
}

impl<'a, 'f> GameCommandClient<'a, 'f> {
    pub fn new(client: &'a Client<'f>) -> Self {
        Self { client }
    }
}

impl CommandClient for GameCommandClient<'_, '_> {
    fn has_local_player(&self) -> bool {
        self.client.local_player().is_some()
    }

    fn send_command(&self, text: &str) -> bool {
        match self.client.send_command(text) {
            Ok(()) => true,
            Err(error) => {
                curia::warn!("the loader refused the command", { error: format!("{error:?}") });
                false
            }
        }
    }
}
