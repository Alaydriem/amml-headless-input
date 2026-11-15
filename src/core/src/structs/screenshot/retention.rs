pub struct ScreenshotRetention;

impl ScreenshotRetention {
    pub const MAX_SCREENSHOTS: usize = 20;

    const PREFIX: &'static str = "shot-";
    const SUFFIX: &'static str = ".png";
    const DIGITS: usize = 13;

    pub fn file_name(unix_millis: u64) -> String {
        format!("{}{:0width$}{}", Self::PREFIX, unix_millis, Self::SUFFIX, width = Self::DIGITS)
    }

    // names are returned oldest first
    pub fn to_delete(names: &[String]) -> Vec<String> {
        let mut ours: Vec<&String> = names.iter().filter(|name| Self::is_ours(name)).collect();
        ours.sort();
        let excess = ours.len().saturating_sub(Self::MAX_SCREENSHOTS);
        ours.into_iter().take(excess).cloned().collect()
    }

    fn is_ours(name: &str) -> bool {
        name.strip_prefix(Self::PREFIX)
            .and_then(|rest| rest.strip_suffix(Self::SUFFIX))
            .is_some_and(|digits| digits.len() == Self::DIGITS && digits.bytes().all(|b| b.is_ascii_digit()))
    }
}
