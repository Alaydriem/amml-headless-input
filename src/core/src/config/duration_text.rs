use crate::control::Ticks;

// Reads duration text for the settings; the rules are those of Ticks.
pub struct DurationText;

impl DurationText {
    pub const TICKS_PER_SECOND: f64 = Ticks::PER_SECOND;
    // one hour
    pub const MAX_TICKS: u32 = Ticks::MAX;

    // the duration in ticks; None for text that Ticks::parse refuses
    pub fn parse_ticks(text: &str) -> Option<u32> {
        Ticks::parse(text).map(Ticks::get)
    }
}
