use alaydriem_headless_input::services::TickWatch;

#[test]
fn the_first_local_tick_of_each_world_is_noted_once() {
    let mut watch = TickWatch::new();
    assert!(!watch.on_frame(true));
    assert!(watch.on_local_tick());
    assert!(!watch.on_local_tick());
    assert!(!watch.on_frame(true));

    assert!(!watch.on_frame(false));
    assert!(!watch.on_frame(true));
    assert!(watch.on_local_tick());
}

#[test]
fn a_tick_before_the_first_world_frame_counts_for_that_world() {
    let mut watch = TickWatch::new();
    assert!(watch.on_local_tick());
    assert!(!watch.on_frame(true));
    assert!(!watch.on_local_tick());
}

#[test]
fn a_world_with_no_local_tick_warns_once_at_the_frame_limit() {
    let mut watch = TickWatch::new();
    let warnings: Vec<u32> = (1..=TickWatch::FRAMES_BEFORE_WARNING * 2).filter(|_| watch.on_frame(true)).collect();
    assert_eq!(warnings.len(), 1);

    let mut watch = TickWatch::new();
    let fired: Vec<u32> = (1..=TickWatch::FRAMES_BEFORE_WARNING).filter(|_| watch.on_frame(true)).collect();
    assert_eq!(fired.len(), 1);
    assert!(!watch.on_frame(false));
    let again = (1..=TickWatch::FRAMES_BEFORE_WARNING).filter(|_| watch.on_frame(true)).count();
    assert_eq!(again, 1);
}

#[test]
fn a_matched_world_never_warns() {
    let mut watch = TickWatch::new();
    watch.on_frame(true);
    watch.on_local_tick();
    assert_eq!((0..TickWatch::FRAMES_BEFORE_WARNING * 2).filter(|_| watch.on_frame(true)).count(), 0);
}
