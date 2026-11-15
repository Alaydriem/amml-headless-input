use alaydriem_headless_input::services::ScreenshotFolder;
use headless_input_core::structs::ScreenshotRetention;

#[test]
fn prune_keeps_the_newest_twenty() {
    let temp = tempfile::tempdir().unwrap();
    let folder = ScreenshotFolder::new(temp.path().to_path_buf());
    for stamp in 1..=25u64 {
        std::fs::write(temp.path().join(ScreenshotRetention::file_name(stamp)), b"x").unwrap();
    }
    std::fs::write(temp.path().join("notes.txt"), b"keep").unwrap();

    folder.prune();

    let mut names: Vec<String> = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names.len(), ScreenshotRetention::MAX_SCREENSHOTS + 1);
    assert!(names.contains(&"notes.txt".to_string()));
    assert!(!names.contains(&ScreenshotRetention::file_name(5)));
    assert!(names.contains(&ScreenshotRetention::file_name(6)));
    assert!(names.contains(&ScreenshotRetention::file_name(25)));
}

#[test]
fn next_path_is_in_the_folder_and_makes_it() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("screenshots");
    let folder = ScreenshotFolder::new(dir.clone());

    let path = folder.next_path(1_700_000_000_123);

    assert!(dir.is_dir());
    assert_eq!(path, dir.join(ScreenshotRetention::file_name(1_700_000_000_123)));
}
