use std::time::Duration;

// A random wait before a member tries to take over, so the members of a lost hub do not all bind at once.
pub struct Jitter;

impl Jitter {
    pub const MAX: Duration = Duration::from_millis(250);

    // a wait from 0 to `MAX`
    pub fn wait() -> Duration {
        let mut bytes = [0u8; 2];
        let random = match getrandom::fill(&mut bytes) {
            Ok(()) => u64::from(u16::from_le_bytes(bytes)),
            Err(_) => u64::from(std::process::id()),
        };
        let max = Self::MAX.as_millis() as u64;
        Duration::from_millis(random % (max + 1))
    }
}
