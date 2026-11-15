mod catalog;
mod instance_data_dir;

use headless_input_core::structs::{WorldEntry, WorldStorage};

pub fn entry(level_id: &str, name: &str, storage: WorldStorage, owner: &str) -> WorldEntry {
    WorldEntry {
        level_id: level_id.to_string(),
        name: name.to_string(),
        storage,
        owner: owner.to_string(),
        last_played: None,
    }
}
