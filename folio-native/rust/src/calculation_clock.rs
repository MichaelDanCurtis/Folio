//! Runtime clock context; never part of the workbook or undo history.
use chrono::{Datelike, Local};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalculationClock {
    pub millis: i64,
    pub offset_seconds: i32,
    pub local_day: i32,
}
impl CalculationClock {
    pub fn system() -> Self {
        let now = Local::now();
        Self {
            millis: now.timestamp_millis(),
            offset_seconds: now.offset().local_minus_utc(),
            local_day: now.num_days_from_ce(),
        }
    }
    pub fn same_calendar(self, other: Self) -> bool {
        self.local_day == other.local_day && self.offset_seconds == other.offset_seconds
    }
    #[cfg(test)]
    pub fn fixed(rfc3339: &str) -> Self {
        let now = chrono::DateTime::parse_from_rfc3339(rfc3339).unwrap();
        Self {
            millis: now.timestamp_millis(),
            offset_seconds: now.offset().local_minus_utc(),
            local_day: now.num_days_from_ce(),
        }
    }
}
