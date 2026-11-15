use headless_input_core::structs::ScreenshotRetention;

#[test]
fn keeps_the_newest_twenty() {
    let names: Vec<String> = (0..25u64).map(|i| ScreenshotRetention::file_name(1_000 + i)).collect();
    let gone = ScreenshotRetention::to_delete(&names);
    assert_eq!(gone, names[..5].to_vec());
}

#[test]
fn sorts_before_choosing_the_oldest() {
    let ordered: Vec<String> = (0..25u64).map(|i| ScreenshotRetention::file_name(1_000 + i)).collect();
    let shuffled: Vec<String> = (0..25usize).map(|i| ordered[(i * 7) % 25].clone()).collect();
    let mut gone = ScreenshotRetention::to_delete(&shuffled);
    gone.sort();
    assert_eq!(gone, ordered[..5].to_vec());
}

#[test]
fn ignores_other_files() {
    let mut names: Vec<String> = (0..25u64).map(|i| ScreenshotRetention::file_name(1_000 + i)).collect();
    names.push("notes.txt".to_string());
    let gone = ScreenshotRetention::to_delete(&names);
    assert_eq!(gone.len(), 5);
    assert!(!gone.contains(&"notes.txt".to_string()));
}

#[test]
fn file_names_sort_by_time() {
    assert!(ScreenshotRetention::file_name(999) < ScreenshotRetention::file_name(1_000_000));
}
