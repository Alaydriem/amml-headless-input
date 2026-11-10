use std::net::Ipv4Addr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use alaydriem_headless_input::join::JoinService;
use headless_input_core::structs::JoinTarget;

use crate::support::fake_join_client::FakeJoinClient;

fn target() -> Arc<JoinTarget> {
    Arc::new(JoinTarget { host: "play.example.net".to_string(), address: Ipv4Addr::new(10, 0, 0, 2), port: 19133 })
}

// the instant every test's clock counts from
fn base() -> Instant {
    static BASE: OnceLock<Instant> = OnceLock::new();
    *BASE.get_or_init(Instant::now)
}

fn at(millis: u64) -> Instant {
    base() + Duration::from_millis(millis)
}

// frames of a join step until the join call: the tab is asked, then the frames run to the end of the wait
fn open_and_wait(service: &mut JoinService, client: &FakeJoinClient, target: &Arc<JoinTarget>) {
    join_frame(service, client, target);
    join_frame_at(service, client, target, at(0) + JoinService::TAB_WAIT);
}

// one frame of a join step at the start of the clock: the frame asks for the join, then the service pumps
fn join_frame(service: &mut JoinService, client: &FakeJoinClient, target: &Arc<JoinTarget>) {
    join_frame_at(service, client, target, at(0));
}

fn join_frame_at(service: &mut JoinService, client: &FakeJoinClient, target: &Arc<JoinTarget>, now: Instant) {
    service.request(Arc::clone(target));
    service.pump(client, now);
}

fn leave_frame(service: &mut JoinService, client: &FakeJoinClient) {
    service.request_leave();
    service.pump(client, at(0));
}

#[test]
fn join_waits_for_the_title_screen() {
    let client = FakeJoinClient::new("world_saving_progress_screen");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    for screen in ["world_saving_progress_screen", "toast_screen", "play_screen", "hud_screen"] {
        client.show(screen);
        join_frame(&mut service, &client, &target);
    }
    assert!(client.joins().is_empty(), "no join off the title screen");

    client.show("start_screen");
    client.set_primary_ready(false);
    join_frame(&mut service, &client, &target);
    client.set_primary_ready(true);
    client.set_in_game(true);
    join_frame(&mut service, &client, &target);
    assert!(client.joins().is_empty(), "no join before the primary client is ready or while in game");

    client.set_in_game(false);
    join_frame(&mut service, &client, &target);
    assert_eq!(client.joins(), [("10.0.0.2".to_string(), 19133, "play.example.net".to_string())]);
}

#[test]
fn join_calls_the_game_once() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    // the title stays on top for a few frames after the call, then the world loads; the step asks every frame
    for _ in 0..5 {
        join_frame(&mut service, &client, &target);
    }
    client.show("world_loading_progress_screen");
    for _ in 0..5 {
        join_frame(&mut service, &client, &target);
    }
    client.show("hud_screen");
    client.set_in_game(true);
    join_frame(&mut service, &client, &target);
    assert_eq!(client.joins().len(), 1);
}

#[test]
fn a_refused_join_retries_next_frame() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    client.answer_next(false);
    let mut service = JoinService::new();
    let target = target();
    join_frame(&mut service, &client, &target);
    assert_eq!(client.joins().len(), 1);
    join_frame(&mut service, &client, &target);
    assert_eq!(client.joins().len(), 2, "the refused join is asked again");
    join_frame(&mut service, &client, &target);
    assert_eq!(client.joins().len(), 2, "the taken join is not asked again");
}

#[test]
fn the_join_is_asked_once() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    join_frame(&mut service, &client, &target);
    client.show("world_loading_progress_screen");
    join_frame(&mut service, &client, &target);
    // the join failed and the menus are back: the same request is not asked again
    client.show("start_screen");
    for _ in 0..5 {
        join_frame(&mut service, &client, &target);
    }
    assert_eq!(client.joins().len(), 1);
}

#[test]
fn a_new_request_after_a_failed_join_is_asked() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let first = target();
    join_frame(&mut service, &client, &first);
    client.show("world_loading_progress_screen");
    join_frame(&mut service, &client, &first);
    client.show("start_screen");
    join_frame(&mut service, &client, &first);
    assert_eq!(client.joins().len(), 1);

    // a frame that does not ask ends the request; the next join step is a new request
    service.pump(&client, at(0));
    join_frame(&mut service, &client, &target());
    assert_eq!(client.joins().len(), 2);
}

#[test]
fn a_join_no_frame_asks_for_any_more_does_not_start() {
    let client = FakeJoinClient::new("world_saving_progress_screen");
    let mut service = JoinService::new();
    join_frame(&mut service, &client, &target());
    client.show("start_screen");
    service.pump(&client, at(0));
    assert!(client.joins().is_empty(), "a request ends on the first frame that does not renew it");
}

#[test]
fn leave_calls_request_leave_game_once() {
    let client = FakeJoinClient::in_world();
    let mut service = JoinService::new();
    for _ in 0..3 {
        leave_frame(&mut service, &client);
    }
    client.set_in_game(false);
    client.show("world_saving_progress_screen");
    for _ in 0..3 {
        leave_frame(&mut service, &client);
    }
    assert_eq!(client.leaves(), 1);
    client.show("start_screen");
    service.pump(&client, at(0));
    assert_eq!(client.leaves(), 1, "no leave on the title screen");

    // a later leave step in a new world asks the game again
    client.show("hud_screen");
    client.set_in_game(true);
    service.pump(&client, at(0));
    leave_frame(&mut service, &client);
    assert_eq!(client.leaves(), 2);
}

#[test]
fn join_from_a_world_leaves_first() {
    let client = FakeJoinClient::in_world();
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    // a join step that starts in a world asks to leave until the title shows
    for _ in 0..3 {
        leave_frame(&mut service, &client);
    }
    assert_eq!((client.leaves(), client.joins().len()), (1, 0));
    client.set_in_game(false);
    client.show("world_saving_progress_screen");
    leave_frame(&mut service, &client);
    client.show("toast_screen");
    leave_frame(&mut service, &client);
    assert_eq!((client.leaves(), client.joins().len()), (1, 0));
    // then it asks to join until the world shows
    client.show("start_screen");
    join_frame(&mut service, &client, &target);
    join_frame(&mut service, &client, &target);
    assert_eq!((client.leaves(), client.joins().len()), (1, 1));
}

#[test]
fn a_toast_over_the_title_still_counts_as_the_title() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    client.set_primary_ready(false);
    join_frame(&mut service, &client, &target);
    client.show("toast_screen");
    leave_frame(&mut service, &client);
    assert_eq!(client.leaves(), 0, "no leave asked at the title under a toast");

    client.set_primary_ready(true);
    join_frame(&mut service, &client, &target);

    assert_eq!(client.joins().len(), 1, "the join starts under the toast");
}

#[test]
fn a_join_from_a_ui_menu_is_asked() {
    let client = FakeJoinClient::new("ui://./hbui/index.html");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    join_frame(&mut service, &client, &target);
    join_frame(&mut service, &client, &target);
    assert_eq!(client.joins().len(), 1);
}

#[test]
fn a_leave_is_not_asked_on_a_ui_menu() {
    let client = FakeJoinClient::new("ui://./hbui/index.html");
    let mut service = JoinService::new();
    for _ in 0..3 {
        leave_frame(&mut service, &client);
    }
    assert_eq!(client.leaves(), 0);
}

#[test]
fn a_leave_is_asked_under_a_ui_page_over_a_world() {
    let client = FakeJoinClient::in_world();
    client.show("ui://./hbui/index.html");
    let mut service = JoinService::new();
    for _ in 0..3 {
        leave_frame(&mut service, &client);
    }
    assert_eq!(client.leaves(), 1);
}

#[test]
fn the_servers_tab_is_opened_before_the_join() {
    let client = FakeJoinClient::new("start_screen");
    let mut service = JoinService::new();
    let target = target();
    join_frame(&mut service, &client, &target);
    assert_eq!((client.tabs(), client.joins().len()), (1, 0), "the tab opens first");
    open_and_wait(&mut service, &client, &target);
    assert_eq!(client.events(), ["tab", "join"]);
}

#[test]
fn the_join_waits_the_tab_time_after_the_tab_was_asked() {
    let client = FakeJoinClient::new("start_screen");
    let mut service = JoinService::new();
    let target = target();
    join_frame_at(&mut service, &client, &target, at(10_000));
    assert_eq!(client.tabs(), 1);
    join_frame_at(&mut service, &client, &target, at(12_400));
    assert!(client.joins().is_empty(), "still waiting at 2.4 s");
    join_frame_at(&mut service, &client, &target, at(12_500));
    assert_eq!(client.joins().len(), 1, "joined at 2.5 s");
}

#[test]
fn many_frames_do_not_shorten_the_tab_wait() {
    let client = FakeJoinClient::new("start_screen");
    let mut service = JoinService::new();
    let target = target();
    for _ in 0..2000 {
        join_frame_at(&mut service, &client, &target, at(0));
    }
    assert!(client.joins().is_empty(), "the wait is time, not frames");
}

#[test]
fn a_tab_that_does_not_open_joins_on_the_same_pump() {
    let client = FakeJoinClient::new("start_screen");
    client.answer_tab_next(false);
    let mut service = JoinService::new();
    join_frame_at(&mut service, &client, &target(), at(5_000));
    assert_eq!((client.tabs(), client.joins().len()), (1, 1));
}

#[test]
fn a_new_request_asks_the_tab_again_and_waits_again() {
    let client = FakeJoinClient::new("start_screen");
    client.answer_next(false);
    let mut service = JoinService::new();
    let target = target();
    join_frame_at(&mut service, &client, &target, at(0));
    join_frame_at(&mut service, &client, &target, at(2_500));
    assert_eq!((client.tabs(), client.joins().len()), (1, 1));

    // a frame that does not ask ends the request
    service.pump(&client, at(3_000));
    join_frame_at(&mut service, &client, &target, at(4_000));
    assert_eq!((client.tabs(), client.joins().len()), (2, 1), "the tab is asked again");
    join_frame_at(&mut service, &client, &target, at(6_400));
    assert_eq!(client.joins().len(), 1, "waits 2.4 s, not yet");
    join_frame_at(&mut service, &client, &target, at(6_500));
    assert_eq!(client.joins().len(), 2, "joins 2.5 s after the new tab call");
}

#[test]
fn a_new_target_with_no_gap_frame_asks_the_tab_again_and_waits_again() {
    let client = FakeJoinClient::new("start_screen");
    let mut service = JoinService::new();
    let first = target();
    open_and_wait(&mut service, &client, &first);
    assert_eq!((client.tabs(), client.joins().len()), (1, 1));

    // the first join failed back to the title; the next frame already asks for another target
    let second = Arc::new(JoinTarget {
        host: "other.example.net".to_string(),
        address: Ipv4Addr::new(10, 0, 0, 3),
        port: 19132,
    });
    join_frame_at(&mut service, &client, &second, at(3_000));
    assert_eq!((client.tabs(), client.joins().len()), (2, 1), "the tab is asked again for the new target");
    join_frame_at(&mut service, &client, &second, at(5_400));
    assert_eq!(client.joins().len(), 1, "waits 2.4 s, not yet");
    join_frame_at(&mut service, &client, &second, at(5_500));
    assert_eq!(client.joins().len(), 2, "joins 2.5 s after the new tab call");
}

#[test]
fn a_tab_that_does_not_open_does_not_stop_the_join() {
    let client = FakeJoinClient::new("start_screen");
    client.answer_tab_next(false);
    let mut service = JoinService::new();
    let target = target();
    join_frame(&mut service, &client, &target);
    assert_eq!((client.tabs(), client.joins().len()), (1, 1), "the join goes on at once");
    join_frame(&mut service, &client, &target);
    assert_eq!((client.tabs(), client.joins().len()), (1, 1));
}

#[test]
fn a_join_from_a_world_leaves_then_opens_the_tab() {
    let client = FakeJoinClient::in_world();
    let mut service = JoinService::new();
    let target = target();
    for _ in 0..3 {
        leave_frame(&mut service, &client);
    }
    assert_eq!((client.leaves(), client.tabs()), (1, 0), "no tab in a world");
    client.set_in_game(false);
    client.show("start_screen");
    open_and_wait(&mut service, &client, &target);
    assert_eq!((client.leaves(), client.events()), (1, vec!["tab", "join"]));
}

#[test]
fn the_tab_is_not_opened_twice_for_one_join() {
    let client = FakeJoinClient::new("start_screen");
    client.answer_next(false);
    let mut service = JoinService::new();
    let target = target();
    open_and_wait(&mut service, &client, &target);
    for _ in 0..5 {
        join_frame_at(&mut service, &client, &target, at(0) + JoinService::TAB_WAIT);
    }
    assert_eq!((client.tabs(), client.joins().len()), (1, 2), "a refused join retries without the tab");

    // a frame that does not ask ends the request; a new request opens the tab again
    service.pump(&client, at(0));
    join_frame(&mut service, &client, &target);
    assert_eq!(client.tabs(), 2);
}

#[test]
fn a_failure_code_the_game_answers_with_is_reported_once() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    service.request(Arc::clone(&target));
    assert_eq!(service.pump(&client, at(0)), None);
    assert_eq!(client.joins().len(), 1);

    client.answer_with(8);
    service.request(Arc::clone(&target));
    assert_eq!(service.pump(&client, at(10)), Some(8));

    service.request(Arc::clone(&target));
    assert_eq!(service.pump(&client, at(20)), None);
}

#[test]
fn no_answer_reports_nothing_while_the_join_waits() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    let mut service = JoinService::new();
    let target = target();
    for millis in [0, 10, 20] {
        service.request(Arc::clone(&target));
        assert_eq!(service.pump(&client, at(millis)), None);
    }
}

#[test]
fn a_code_left_from_an_earlier_join_is_not_reported_for_a_new_one() {
    let client = FakeJoinClient::new("start_screen");
    client.set_tab_opens(false);
    client.answer_with(9);
    let mut service = JoinService::new();
    let target = target();

    service.request(Arc::clone(&target));
    assert_eq!(service.pump(&client, at(0)), None);
    service.request(Arc::clone(&target));
    assert_eq!(service.pump(&client, at(10)), None);
    assert_eq!(client.joins().len(), 1);
}
