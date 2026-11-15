// Unix seconds as ISO 8601 UTC text.
pub struct IsoTime;

impl IsoTime {
    // `2026-11-15T17:47:25Z`, from the civil-date algorithm over days since 1970-01-01
    pub fn format(unix_secs: u64) -> String {
        let days = i64::try_from(unix_secs / 86_400).unwrap_or(i64::MAX / 2);
        let secs = unix_secs % 86_400;
        let (year, month, day) = Self::civil(days);
        format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", secs / 3600, secs % 3600 / 60, secs % 60)
    }

    fn civil(days: i64) -> (i64, i64, i64) {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        (year, month, day)
    }
}
