use headless_input_core::structs::{WorldCatalog, WorldStorage};

use super::entry;

#[test]
fn level_id_match_beats_name_match() {
    let catalog =
        WorldCatalog::new(vec![entry("A", "B2", WorldStorage::User, "1"), entry("B2", "x", WorldStorage::User, "1")]);
    assert_eq!(catalog.find("B2").map(|w| w.level_id.as_str()), Some("B2"));
}

#[test]
fn name_match_ignores_case_and_end_whitespace() {
    let catalog = WorldCatalog::new(vec![entry("id1", "Test Flat", WorldStorage::User, "1")]);
    assert_eq!(catalog.find("  test flat ").map(|w| w.level_id.as_str()), Some("id1"));
}

#[test]
fn first_match_follows_search_order_user_before_shared_then_level_id() {
    let catalog = WorldCatalog::new(vec![
        entry("zz", "Escape room", WorldStorage::Shared, "Shared"),
        entry("bb", "Escape room", WorldStorage::User, "2"),
        entry("aa", "Escape room", WorldStorage::User, "2"),
    ]);
    assert_eq!(catalog.find("Escape room").map(|w| w.level_id.as_str()), Some("aa"));
    assert_eq!(catalog.entries().last().map(|w| w.level_id.as_str()), Some("zz"));
}

#[test]
fn no_match_gives_none() {
    let catalog = WorldCatalog::new(vec![entry("id1", "Test Flat", WorldStorage::User, "1")]);
    assert!(catalog.find("Other").is_none());
}

#[test]
fn blank_text_finds_nothing_even_when_a_world_has_no_name() {
    let catalog = WorldCatalog::new(vec![entry("id1", "", WorldStorage::User, "1")]);
    assert!(catalog.find("").is_none());
    assert!(catalog.find("   ").is_none());
}

#[test]
fn catalog_keeps_at_most_max_worlds_in_search_order() {
    let entries =
        (0..WorldCatalog::MAX_WORLDS + 5).map(|i| entry(&format!("{i:05}"), "w", WorldStorage::User, "1")).collect();
    let catalog = WorldCatalog::new(entries);
    assert_eq!(catalog.entries().len(), WorldCatalog::MAX_WORLDS);
    assert_eq!(catalog.entries()[0].level_id, "00000");
    assert!(catalog.find(&format!("{:05}", WorldCatalog::MAX_WORLDS + 1)).is_none());
}

#[test]
fn name_from_bytes_takes_first_line_without_bom_or_cr() {
    assert_eq!(WorldCatalog::name_from_bytes(b"\xEF\xBB\xBFTest Flat\r\nsecond"), "Test Flat");
}

#[test]
fn name_from_bytes_reads_at_most_max_name_bytes() {
    let long = vec![b'a'; WorldCatalog::MAX_NAME_BYTES + 50];
    assert_eq!(WorldCatalog::name_from_bytes(&long).len(), WorldCatalog::MAX_NAME_BYTES);
}
