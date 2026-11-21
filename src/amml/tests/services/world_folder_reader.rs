use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use alaydriem_headless_input::services::WorldFolderReader;
use headless_input_core::structs::{WorldCatalog, WorldStorage};

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self { dir: tempfile::tempdir().unwrap() }
    }

    fn game(&self) -> PathBuf {
        self.dir.path().join("game")
    }

    fn fallback(&self) -> PathBuf {
        self.dir.path().join("real-appdata")
    }

    fn reader(&self) -> WorldFolderReader {
        WorldFolderReader::new(self.game(), self.fallback())
    }

    fn instance_hcl(&self, text: &str) {
        let file = self.game().join("amml").join("instance.hcl");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    fn manifest(&self, identity: &str) {
        fs::create_dir_all(self.game()).unwrap();
        let text = format!("<Package><Identity Name=\"{identity}\" Version=\"1.0.0.0\" /></Package>");
        fs::write(self.game().join("appxmanifest.xml"), text).unwrap();
    }

    fn world(root: &Path, owner: &str, id: &str, name: Option<&str>, level_dat: bool) {
        Self::world_in(root, "Minecraft Bedrock", owner, id, name, level_dat);
    }

    fn world_in(root: &Path, product: &str, owner: &str, id: &str, name: Option<&str>, level_dat: bool) {
        let folder = root
            .join(product)
            .join("Users")
            .join(owner)
            .join("games")
            .join("com.mojang")
            .join("minecraftWorlds")
            .join(id);
        fs::create_dir_all(&folder).unwrap();
        if level_dat {
            fs::write(folder.join("level.dat"), b"x").unwrap();
        }
        if let Some(name) = name {
            fs::write(folder.join("levelname.txt"), name).unwrap();
        }
    }
}

#[test]
fn root_comes_from_instance_hcl() {
    let fixture = Fixture::new();
    fixture.instance_hcl("data_dir = \"D:/instance-data\"\n");
    assert_eq!(fixture.reader().root(), PathBuf::from("D:/instance-data"));
}

#[test]
fn relative_data_dir_is_under_the_game_folder() {
    let fixture = Fixture::new();
    fixture.instance_hcl("data_dir = \"data\"\n");
    assert_eq!(fixture.reader().root(), fixture.game().join("data"));
}

#[test]
fn missing_or_bad_instance_hcl_gives_the_fallback_root() {
    let fixture = Fixture::new();
    assert_eq!(fixture.reader().root(), fixture.fallback());
    fixture.instance_hcl("data_dir = 3\n");
    assert_eq!(fixture.reader().root(), fixture.fallback());
    fixture.instance_hcl("data_dir = = {{\n");
    assert_eq!(fixture.reader().root(), fixture.fallback());
}

#[test]
fn reads_user_and_shared_worlds_with_names() {
    let fixture = Fixture::new();
    fixture.instance_hcl("data_dir = \"data\"\n");
    let root = fixture.game().join("data");
    Fixture::world(&root, "123", "id1", Some("Test Flat"), true);
    Fixture::world(&root, "Shared", "id2", Some("Common\r\nsecond line"), true);

    let catalog = fixture.reader().read();

    assert_eq!(catalog.entries().len(), 2);
    let user = catalog.find("id1").unwrap();
    assert_eq!((user.name.as_str(), user.storage, user.owner.as_str()), ("Test Flat", WorldStorage::User, "123"));
    let shared = catalog.find("id2").unwrap();
    assert_eq!(
        (shared.name.as_str(), shared.storage, shared.owner.as_str()),
        ("Common", WorldStorage::Shared, "Shared")
    );
}

#[test]
fn folder_without_level_dat_is_skipped() {
    let fixture = Fixture::new();
    Fixture::world(&fixture.fallback(), "123", "id1", Some("Broken"), false);
    assert!(fixture.reader().read().entries().is_empty());
}

#[test]
fn missing_levelname_gives_empty_name_and_id_still_matches() {
    let fixture = Fixture::new();
    Fixture::world(&fixture.fallback(), "123", "id1", None, true);
    let catalog = fixture.reader().read();
    assert_eq!(catalog.find("id1").unwrap().name, "");
}

#[test]
fn no_worlds_folder_gives_an_empty_catalog() {
    let fixture = Fixture::new();
    assert!(fixture.reader().read().entries().is_empty());
}

#[test]
fn reader_stops_after_max_worlds() {
    let fixture = Fixture::new();
    for index in 0..WorldCatalog::MAX_WORLDS + 3 {
        Fixture::world(&fixture.fallback(), "123", &format!("w{index}"), None, true);
    }
    assert_eq!(fixture.reader().read().entries().len(), WorldCatalog::MAX_WORLDS);
}

#[test]
fn last_played_is_the_level_dat_modified_time() {
    let fixture = Fixture::new();
    Fixture::world(&fixture.fallback(), "123", "id1", None, true);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let played = fixture.reader().read().find("id1").unwrap().last_played.unwrap();
    assert!(played.abs_diff(now) < 10, "{played} vs {now}");
}

#[test]
fn a_long_levelname_is_cut_at_max_name_bytes() {
    let fixture = Fixture::new();
    Fixture::world(&fixture.fallback(), "123", "id1", Some(&"a".repeat(2048)), true);
    let name = fixture.reader().read().find("id1").unwrap().name.clone();
    assert_eq!(name.len(), WorldCatalog::MAX_NAME_BYTES);
}

#[test]
fn a_lowercase_shared_owner_is_shared_storage() {
    let fixture = Fixture::new();
    Fixture::world(&fixture.fallback(), "shared", "id1", None, true);
    assert_eq!(fixture.reader().read().find("id1").unwrap().storage, WorldStorage::Shared);
}

#[test]
fn an_empty_fallback_root_reads_nothing() {
    let fixture = Fixture::new();
    let reader = WorldFolderReader::new(fixture.game(), PathBuf::new());
    assert!(reader.root().as_os_str().is_empty());
    assert!(reader.read().entries().is_empty());
}

#[test]
fn a_preview_game_reads_the_preview_worlds_only() {
    let fixture = Fixture::new();
    fixture.manifest("Microsoft.MinecraftWindowsBeta");
    Fixture::world_in(&fixture.fallback(), "Minecraft Bedrock Preview", "123", "preview", None, true);
    Fixture::world_in(&fixture.fallback(), "Minecraft Bedrock", "123", "release", None, true);
    let catalog = fixture.reader().read();
    assert!(catalog.find("preview").is_some());
    assert!(catalog.find("release").is_none());
}

#[test]
fn a_release_game_reads_the_release_worlds_only() {
    let fixture = Fixture::new();
    fixture.manifest("Microsoft.MinecraftUWP");
    Fixture::world_in(&fixture.fallback(), "Minecraft Bedrock Preview", "123", "preview", None, true);
    Fixture::world_in(&fixture.fallback(), "Minecraft Bedrock", "123", "release", None, true);
    let catalog = fixture.reader().read();
    assert!(catalog.find("release").is_some());
    assert!(catalog.find("preview").is_none());
}

#[test]
fn without_a_manifest_both_products_are_read() {
    let fixture = Fixture::new();
    Fixture::world_in(&fixture.fallback(), "Minecraft Bedrock Preview", "123", "preview", None, true);
    Fixture::world_in(&fixture.fallback(), "Minecraft Bedrock", "123", "release", None, true);
    let catalog = fixture.reader().read();
    assert!(catalog.find("release").is_some() && catalog.find("preview").is_some());
}
