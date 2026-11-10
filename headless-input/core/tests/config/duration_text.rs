use headless_input_core::config::DurationText;

#[test]
fn seconds_are_twenty_ticks_each() {
    assert_eq!(DurationText::parse_ticks("5s"), Some(100));
    assert_eq!(DurationText::parse_ticks("30s"), Some(600));
}

#[test]
fn ticks_are_taken_as_they_are() {
    assert_eq!(DurationText::parse_ticks("7t"), Some(7));
}

#[test]
fn decimal_seconds_round_to_the_nearest_tick() {
    assert_eq!(DurationText::parse_ticks("0.12s"), Some(2));
    assert_eq!(DurationText::parse_ticks("0.08s"), Some(2));
    assert_eq!(DurationText::parse_ticks("0.02s"), None);
}

#[test]
fn text_that_is_not_a_duration_is_refused() {
    for text in ["soon", "", "5", "s", "-3s", "NaNs", "infs", "0s", "0t", "1m"] {
        assert_eq!(DurationText::parse_ticks(text), None, "{text}");
    }
}

#[test]
fn a_duration_over_the_cap_is_refused() {
    assert_eq!(DurationText::parse_ticks("3600s"), Some(DurationText::MAX_TICKS));
    assert_eq!(DurationText::parse_ticks("3601s"), None);
}
