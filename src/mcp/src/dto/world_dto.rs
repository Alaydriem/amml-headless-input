use headless_input_core::structs::{WorldEntry, WorldStorage};
use serde::{Deserialize, Serialize};

use super::IsoTime;

// The JSON form of a `WorldEntry`, an item of the `list_worlds` reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDto {
    pub name: String,
    pub level_id: String,
    // "user" or "shared"
    pub storage: String,
    // ISO 8601 UTC
    pub last_played: Option<String>,
}

impl From<&WorldEntry> for WorldDto {
    fn from(entry: &WorldEntry) -> Self {
        let storage = match entry.storage {
            WorldStorage::User => "user",
            WorldStorage::Shared => "shared",
        };
        Self {
            name: entry.name.clone(),
            level_id: entry.level_id.clone(),
            storage: storage.to_owned(),
            last_played: entry.last_played.map(IsoTime::format),
        }
    }
}
