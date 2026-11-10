use std::time::Duration;

// A length of game time in ticks. Duration text: "Ns" is N seconds (integers or decimals) and "Nt" is N ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticks(pub u32);

impl Ticks {
    pub const PER_SECOND: f64 = 20.0;
    // one hour
    pub const MAX: u32 = 72_000;

    pub fn get(self) -> u32 {
        self.0
    }

    // the wall-clock time the ticks take at PER_SECOND
    pub fn duration(self) -> Duration {
        Duration::from_secs_f64(f64::from(self.0) / Self::PER_SECOND)
    }

    // the duration in ticks, seconds rounded to the nearest tick; None for text that is not a duration, a duration
    // of no ticks or one over MAX
    pub fn parse(text: &str) -> Option<Ticks> {
        let ticks = if let Some(seconds) = text.strip_suffix('s') {
            let value: f64 = seconds.parse().ok()?;
            if !value.is_finite() || value < 0.0 {
                return None;
            }
            (value * Self::PER_SECOND).round()
        } else {
            let count: u32 = text.strip_suffix('t')?.parse().ok()?;
            f64::from(count)
        };
        if ticks < 1.0 || ticks > f64::from(Self::MAX) {
            return None;
        }
        Some(Ticks(ticks as u32))
    }
}
