use headless_input_core::control::Ticks;

#[test]
fn seconds_become_twenty_ticks_each() {
    assert_eq!(Ticks::parse("5s"), Some(Ticks(100)));
}

#[test]
fn tick_suffix_is_exact() {
    assert_eq!(Ticks::parse("12t"), Some(Ticks(12)));
}

#[test]
fn decimal_seconds_round_to_the_nearest_tick() {
    assert_eq!(Ticks::parse("0.12s"), Some(Ticks(2)));
}

#[test]
fn negative_or_empty_is_refused() {
    for text in ["-1s", "-3t", "", "s", "t"] {
        assert_eq!(Ticks::parse(text), None, "{text}");
    }
}

#[test]
fn no_unit_is_refused() {
    assert_eq!(Ticks::parse("5"), None);
}
