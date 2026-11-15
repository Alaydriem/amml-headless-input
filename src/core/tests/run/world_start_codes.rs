use headless_input_core::run::WorldStartCodes;

#[test]
fn ok_and_not_ready_are_no_failure() {
    assert_eq!(WorldStartCodes::failure(WorldStartCodes::OK), None);
    assert_eq!(WorldStartCodes::failure(WorldStartCodes::NOT_READY), None);
}

#[test]
fn is_failure_agrees_with_failure() {
    for code in -1..20 {
        assert_eq!(WorldStartCodes::is_failure(code), WorldStartCodes::failure(code).is_some(), "{code}");
    }
}

#[test]
fn each_named_code_names_itself_and_its_number() {
    for (code, text) in [
        (1, "not in the game's world list"),
        (2, "another world start is in progress"),
        (9, "saved in a newer version"),
        (12, "platform locked content"),
        (13, "not found in the file system"),
        (14, "slim cache data"),
    ] {
        let why = WorldStartCodes::failure(code).expect("a failure");
        assert!(why.starts_with("the world join failed: "), "{why}");
        assert!(why.contains(text), "{why}");
        assert!(why.contains(&format!("(result {code})")), "{why}");
    }
}

#[test]
fn any_other_code_is_a_refusal() {
    for code in [4, 8, 10, 11, 15, 99, -1, i32::MAX, i32::MIN] {
        let why = WorldStartCodes::failure(code).expect("a failure");
        assert_eq!(why, format!("the world join failed: the game refused it (result {code})"));
    }
}

#[test]
fn only_not_ready_means_ask_again() {
    assert!(WorldStartCodes::is_not_ready(3));
    for code in [0, 1, 2, 4, -3] {
        assert!(!WorldStartCodes::is_not_ready(code), "{code}");
    }
}
