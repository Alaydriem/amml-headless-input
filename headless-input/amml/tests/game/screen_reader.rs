use alaydriem_headless_input::game::ScreenReader;
use headless_input_core::player::ScreenKind;

#[test]
fn hud_screen_is_in_world() {
    assert_eq!(ScreenReader::kind("hud_screen", &ScreenKind::Title, false), ScreenKind::InWorld);
}

#[test]
fn start_screen_is_title() {
    assert_eq!(ScreenReader::kind("start_screen", &ScreenKind::InWorld, false), ScreenKind::Title);
}

#[test]
fn a_ui_menu_counts_as_the_title() {
    assert_eq!(ScreenReader::kind("ui://./hbui/index.html", &ScreenKind::InWorld, false), ScreenKind::Title);
    assert_eq!(ScreenReader::kind("ui://./hbui/index.html", &ScreenKind::Loading, false), ScreenKind::Title);
}

#[test]
fn a_toast_over_a_ui_menu_keeps_title() {
    let menu = ScreenReader::kind("ui://./hbui/index.html", &ScreenKind::InWorld, false);
    assert_eq!(ScreenReader::kind(ScreenReader::TOAST, &menu, false), ScreenKind::Title);
}

#[test]
fn a_progress_screen_is_loading() {
    assert_eq!(ScreenReader::kind("world_loading_progress_screen", &ScreenKind::Title, false), ScreenKind::Loading);
    assert_eq!(ScreenReader::kind("progress_screen", &ScreenKind::Title, false), ScreenKind::Loading);
}

#[test]
fn pause_screen_is_other() {
    assert_eq!(
        ScreenReader::kind("pause_screen", &ScreenKind::InWorld, false),
        ScreenKind::Other("pause_screen".to_string())
    );
    assert_eq!(ScreenReader::kind("", &ScreenKind::InWorld, false), ScreenKind::Other(String::new()));
}

#[test]
fn a_toast_keeps_the_screen_under_it() {
    let under = [ScreenKind::InWorld, ScreenKind::Title, ScreenKind::Loading, ScreenKind::Other("chat_screen".into())];
    for previous in under {
        assert_eq!(ScreenReader::kind(ScreenReader::TOAST, &previous, false), previous);
    }
}

#[test]
fn a_ui_page_over_a_world_is_other() {
    let page = "ui://./hbui/index.html";
    assert_eq!(ScreenReader::kind(page, &ScreenKind::InWorld, true), ScreenKind::Other(page.to_string()));
    assert_eq!(ScreenReader::kind(page, &ScreenKind::Title, true), ScreenKind::Other(page.to_string()));
    assert_eq!(ScreenReader::kind("start_screen", &ScreenKind::InWorld, true), ScreenKind::Title);
}

#[test]
fn a_ui_page_is_the_title_only_without_a_world() {
    assert!(ScreenReader::is_title("ui://./hbui/index.html", false));
    assert!(!ScreenReader::is_title("ui://./hbui/index.html", true));
    assert!(ScreenReader::is_title("start_screen", false));
    assert!(!ScreenReader::is_title("hud_screen", false));
}
