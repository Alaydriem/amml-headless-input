//! `LegacyTokenFile`: the one-time copy of the token file from the folder older builds used.

use headless_input_mcp::token::{LegacyCopy, LegacyTokenFile, TokenStore};

#[test]
fn the_old_folder_is_amml_headless_input_under_local_app_data() {
    let folder = LegacyTokenFile::folder_in(std::path::Path::new(r"C:\Users\x\AppData\Local"));

    assert_eq!(folder, std::path::PathBuf::from(r"C:\Users\x\AppData\Local\amml\headless-input"));
}

#[test]
fn a_token_paired_in_the_old_folder_opens_the_new_one_and_the_old_file_stays() {
    let legacy = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("mod-data").join("@alaydriem").join("headless-input");
    let token = TokenStore::load(legacy.path()).issue("claude-code").unwrap();

    assert_eq!(LegacyTokenFile::copy_once(legacy.path(), &folder).unwrap(), LegacyCopy::Copied);

    assert!(TokenStore::load(&folder).contains(&token));
    assert!(TokenStore::load(legacy.path()).contains(&token));
    let left: Vec<_> = std::fs::read_dir(&folder).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(left, vec![std::ffi::OsString::from(TokenStore::FILE_NAME)]);
}

#[test]
fn a_new_folder_with_its_own_token_file_is_not_overwritten() {
    let legacy = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    let old = TokenStore::load(legacy.path()).issue("old").unwrap();
    let new = TokenStore::load(folder.path()).issue("new").unwrap();

    assert_eq!(LegacyTokenFile::copy_once(legacy.path(), folder.path()).unwrap(), LegacyCopy::AlreadyPresent);

    let store = TokenStore::load(folder.path());
    assert!(store.contains(&new));
    assert!(!store.contains(&old));
}

#[test]
fn no_old_file_copies_nothing_and_makes_no_folder() {
    let legacy = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("headless-input");

    assert_eq!(LegacyTokenFile::copy_once(legacy.path(), &folder).unwrap(), LegacyCopy::NoLegacyFile);

    assert!(!folder.exists());
}

#[test]
fn a_second_copy_after_a_new_token_keeps_the_new_token() {
    let legacy = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    TokenStore::load(legacy.path()).issue("old").unwrap();
    LegacyTokenFile::copy_once(legacy.path(), folder.path()).unwrap();
    let fresh = TokenStore::load(folder.path()).issue("fresh").unwrap();

    assert_eq!(LegacyTokenFile::copy_once(legacy.path(), folder.path()).unwrap(), LegacyCopy::AlreadyPresent);

    assert!(TokenStore::load(folder.path()).contains(&fresh));
}
