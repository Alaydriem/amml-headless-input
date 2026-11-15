//! ISO 8601 text from Unix seconds, at the boundaries that break hand-written calendars.

use headless_input_mcp::dto::IsoTime;

#[test]
fn the_epoch_is_the_first_second_of_1970() {
    assert_eq!(IsoTime::format(0), "1970-01-01T00:00:00Z");
}

#[test]
fn a_leap_day_is_kept() {
    assert_eq!(IsoTime::format(1_709_251_199), "2024-02-29T23:59:59Z");
    assert_eq!(IsoTime::format(1_709_251_200), "2024-03-01T00:00:00Z");
}

#[test]
fn a_year_end_rolls_into_january() {
    assert_eq!(IsoTime::format(1_735_689_599), "2024-12-31T23:59:59Z");
    assert_eq!(IsoTime::format(1_735_689_600), "2025-01-01T00:00:00Z");
}

#[test]
fn a_century_year_that_is_not_a_leap_year_has_no_february_29() {
    // 2100-03-01T00:00:00Z
    assert_eq!(IsoTime::format(4_107_542_400), "2100-03-01T00:00:00Z");
}
