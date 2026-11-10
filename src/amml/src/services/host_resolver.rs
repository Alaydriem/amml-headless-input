use amml_game::EntryResolver;
use amml_sdk::SdkError;

use crate::traits::GameHost;

// the loader a game host stands in for, as the resolver the game entries are read with
pub struct HostResolver<'a, H: GameHost>(pub &'a H);

impl<H: GameHost> EntryResolver for HostResolver<'_, H> {
    fn resolve(&self, id: &str) -> Result<u64, SdkError> {
        self.0.resolve(id)
    }
}
