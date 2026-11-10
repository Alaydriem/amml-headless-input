use alaydriem_headless_input::services::LastView;
use alaydriem_headless_input::types::FrameView;
use headless_input_core::player::{PlayerView, ScreenKind};

fn frame(position: [f32; 3], dimension: Option<i32>, screen: ScreenKind, fresh: bool) -> FrameView {
    FrameView { view: PlayerView { position, yaw: 10.0, pitch: 5.0, dimension, screen }, in_world: true, fresh }
}

#[test]
fn a_failed_read_takes_the_last_good_view_and_keeps_its_own_screen() {
    let mut last = LastView::new();
    let mut good = frame([1.0, 2.0, 3.0], Some(1), ScreenKind::InWorld, true);
    last.settle(&mut good);
    assert_eq!(good.view.dimension, Some(1));

    let mut failed = frame([0.0; 3], None, ScreenKind::Loading, false);
    failed.view.yaw = 0.0;
    failed.view.pitch = 0.0;
    last.settle(&mut failed);

    assert_eq!(failed.view.position, [1.0, 2.0, 3.0]);
    assert_eq!((failed.view.yaw, failed.view.pitch), (10.0, 5.0));
    assert_eq!(failed.view.dimension, Some(1));
    assert_eq!(failed.view.screen, ScreenKind::Loading);
    assert!(failed.in_world);
}

#[test]
fn a_failed_read_before_any_good_one_and_after_a_clear_stays_as_read() {
    let mut last = LastView::new();
    let mut failed = frame([0.0; 3], None, ScreenKind::InWorld, false);
    last.settle(&mut failed);
    assert_eq!(failed.view.dimension, None);

    last.settle(&mut frame([1.0, 2.0, 3.0], Some(2), ScreenKind::InWorld, true));
    last.clear();
    let mut failed = frame([0.0; 3], None, ScreenKind::InWorld, false);
    last.settle(&mut failed);
    assert_eq!(failed.view.position, [0.0; 3]);
    assert_eq!(failed.view.dimension, None);
}

#[test]
fn a_frame_outside_a_world_drops_the_held_view_and_the_dimension_stays_unknown() {
    let mut last = LastView::new();
    last.settle(&mut frame([1.0, 2.0, 3.0], Some(1), ScreenKind::InWorld, true));

    let mut title = frame([0.0; 3], None, ScreenKind::Title, false);
    title.in_world = false;
    last.settle(&mut title);
    assert_eq!(title.view.dimension, None);

    let mut failed = frame([0.0; 3], None, ScreenKind::InWorld, false);
    failed.view.yaw = 0.0;
    failed.view.pitch = 0.0;
    last.settle(&mut failed);
    assert_eq!(failed.view.position, [0.0; 3]);
    assert_eq!((failed.view.yaw, failed.view.pitch), (0.0, 0.0));
    assert_eq!(failed.view.dimension, None);
}

#[test]
fn the_screen_of_the_last_frame_is_kept_for_a_toast() {
    let mut last = LastView::new();
    assert_eq!(last.screen(), &ScreenKind::Title);

    last.settle(&mut frame([1.0, 2.0, 3.0], Some(0), ScreenKind::InWorld, true));
    assert_eq!(last.screen(), &ScreenKind::InWorld);
    let mut outside = frame([0.0; 3], None, ScreenKind::Loading, false);
    outside.in_world = false;
    last.settle(&mut outside);
    assert_eq!(last.screen(), &ScreenKind::Loading);

    last.clear();
    assert_eq!(last.screen(), &ScreenKind::Title);
}
