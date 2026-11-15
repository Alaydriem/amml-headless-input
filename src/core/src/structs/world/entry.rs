use super::WorldStorage;

// One local world found on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldEntry {
    pub level_id: String,
    pub name: String,
    pub storage: WorldStorage,
    // The folder under `Users` that holds the world.
    pub owner: String,
    // Unix seconds.
    pub last_played: Option<u64>,
}
