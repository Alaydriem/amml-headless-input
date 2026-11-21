use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use headless_input_core::structs::{InstanceDataDir, WorldCatalog, WorldEntry, WorldStorage};

// Finds the worlds the game sees. The loader moves the game's `%APPDATA%` to the `data_dir` of
// `<game>\amml\instance.hcl`, but only for the game's own imports, so the mod follows the same rule itself
pub struct WorldFolderReader {
    game_dir: PathBuf,
    fallback_root: PathBuf,
}

impl WorldFolderReader {
    const INSTANCE_FILE: &'static [&'static str] = &["amml", "instance.hcl"];
    const WORLDS: &'static [&'static str] = &["games", "com.mojang", "minecraftWorlds"];
    const SHARED: &'static str = "Shared";
    const MANIFEST: &'static str = "appxmanifest.xml";
    const RELEASE: &'static str = "Minecraft Bedrock";
    const PREVIEW: &'static str = "Minecraft Bedrock Preview";

    pub fn new(game_dir: PathBuf, fallback_root: PathBuf) -> Self {
        Self { game_dir, fallback_root }
    }

    // the data root: the `data_dir` of `instance.hcl`, else the fallback
    pub fn root(&self) -> PathBuf {
        let file = Self::INSTANCE_FILE.iter().fold(self.game_dir.clone(), |path, part| path.join(part));
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return self.fallback_root.clone(),
            Err(error) => {
                curia::warn!("instance.hcl could not be read; the real app data folder is used", {
                    error: error.to_string()
                });
                return self.fallback_root.clone();
            }
        };
        match InstanceDataDir::parse(&text, &self.game_dir) {
            Some(dir) => dir,
            None => {
                curia::warn!("instance.hcl has no usable data_dir; the real app data folder is used");
                self.fallback_root.clone()
            }
        }
    }

    // every world folder under the root, at most `MAX_WORLDS`; a folder that cannot be read is skipped
    pub fn read(&self) -> WorldCatalog {
        let mut entries = Vec::new();
        let root = self.root();
        if root.as_os_str().is_empty() {
            return WorldCatalog::new(entries);
        }
        for product in self.products() {
            let Ok(owners) = fs::read_dir(root.join(product).join("Users")) else {
                continue;
            };
            if !Self::read_owners(owners, &mut entries) {
                break;
            }
        }
        WorldCatalog::new(entries)
    }

    // the product folders the game keeps its data in: from the package identity in the game's
    // `appxmanifest.xml`; both when the manifest is missing or names neither
    fn products(&self) -> Vec<&'static str> {
        let identity = fs::read_to_string(self.game_dir.join(Self::MANIFEST)).unwrap_or_default();
        if identity.contains("Name=\"Microsoft.MinecraftWindowsBeta\"") {
            vec![Self::PREVIEW]
        } else if identity.contains("Name=\"Microsoft.MinecraftUWP\"") {
            vec![Self::RELEASE]
        } else {
            vec![Self::RELEASE, Self::PREVIEW]
        }
    }

    // adds the worlds of every owner folder; `false` once `MAX_WORLDS` is reached
    fn read_owners(owners: fs::ReadDir, entries: &mut Vec<WorldEntry>) -> bool {
        for owner in owners.flatten() {
            let owner_name = owner.file_name().to_string_lossy().into_owned();
            let storage =
                if owner_name.eq_ignore_ascii_case(Self::SHARED) { WorldStorage::Shared } else { WorldStorage::User };
            let worlds = Self::WORLDS.iter().fold(owner.path(), |path, part| path.join(part));
            let Ok(levels) = fs::read_dir(worlds) else {
                continue;
            };
            for level in levels.flatten() {
                if entries.len() >= WorldCatalog::MAX_WORLDS {
                    return false;
                }
                if let Some(entry) = Self::entry(&level.path(), &owner_name, storage) {
                    entries.push(entry);
                }
            }
        }
        true
    }

    fn entry(folder: &Path, owner: &str, storage: WorldStorage) -> Option<WorldEntry> {
        let modified = fs::metadata(folder.join("level.dat")).ok()?.modified().ok();
        let level_id = folder.file_name()?.to_string_lossy().into_owned();
        let last_played = modified.and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|age| age.as_secs());
        Some(WorldEntry {
            level_id,
            name: Self::name(&folder.join("levelname.txt")),
            storage,
            owner: owner.to_string(),
            last_played,
        })
    }

    fn name(file: &Path) -> String {
        let Ok(opened) = File::open(file) else {
            return String::new();
        };
        let mut bytes = Vec::with_capacity(WorldCatalog::MAX_NAME_BYTES);
        match opened.take(WorldCatalog::MAX_NAME_BYTES as u64).read_to_end(&mut bytes) {
            Ok(_) => WorldCatalog::name_from_bytes(&bytes),
            Err(_) => String::new(),
        }
    }
}
