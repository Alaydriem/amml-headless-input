use amm_client::link::PointerOp;
use amml_api::{Client, MoveIntent};

use crate::errors::{LeaseError, OutputError};
use crate::traits::ManagerPort;

// The manager link of one game call, with the client of the event: input goes through the link, the hotbar select
// through the client's HUD. Lives for one listener call. Game thread
pub struct GamePort<'a, 'f, P: ManagerPort> {
    link: &'a P,
    client: Option<&'a Client<'f>>,
}

impl<'a, 'f, P: ManagerPort> GamePort<'a, 'f, P> {
    pub fn new(link: &'a P, client: Option<&'a Client<'f>>) -> Self {
        Self { link, client }
    }
}

impl<P: ManagerPort> ManagerPort for GamePort<'_, '_, P> {
    fn acquire_input(&self) -> Result<(), LeaseError> {
        self.link.acquire_input()
    }

    fn release_input(&self) {
        self.link.release_input();
    }

    fn send_key(&self, vk: u32, down: bool) -> Result<(), OutputError> {
        self.link.send_key(vk, down)
    }

    fn send_pointer(&self, op: PointerOp) -> Result<(), OutputError> {
        self.link.send_pointer(op)
    }

    fn send_move(&self, intent: MoveIntent) -> Result<(), OutputError> {
        self.link.send_move(intent)
    }

    // the game's own hotbar select; one log line for each slot the game took
    fn select_hotbar(&self, index: u8) -> Result<(), OutputError> {
        let client = self.client.ok_or(OutputError::NotReady)?;
        client.hud().select_hotbar_slot(index).map_err(OutputError::from_api)?;
        curia::info!("hotbar selected", { slot: u16::from(index) + 1 });
        Ok(())
    }

    fn panel_open(&self) -> bool {
        self.link.panel_open()
    }
}
