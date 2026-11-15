use std::path::{Path, PathBuf};

use headless_input_core::structs::InstanceDataDir;

#[test]
fn absolute_data_dir_is_used() {
    let game = Path::new("C:\\game");
    assert_eq!(InstanceDataDir::parse("data_dir = \"D:\\\\data\"", game), Some(PathBuf::from("D:\\data")));
}

#[test]
fn relative_data_dir_is_resolved_against_the_game_folder() {
    assert_eq!(
        InstanceDataDir::parse("data_dir = \"data\"", Path::new("C:\\game")),
        Some(PathBuf::from("C:\\game\\data"))
    );
}

#[test]
fn missing_bad_type_or_bad_hcl_gives_none() {
    let game = Path::new("C:\\game");
    assert_eq!(InstanceDataDir::parse("other = 1", game), None);
    assert_eq!(InstanceDataDir::parse("data_dir = 3", game), None);
    assert_eq!(InstanceDataDir::parse("data_dir = ", game), None);
}
