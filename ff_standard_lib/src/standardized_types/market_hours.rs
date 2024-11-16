use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc, Weekday};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct DaySession {
    pub open: Option<NaiveTime>,
    pub close: Option<NaiveTime>,
}

impl DaySession {
    pub fn is_trading_time(&self, time: NaiveTime) -> bool {
        match (self.open, self.close) {
            (Some(open), Some(close)) if close > open => time >= open && time < close, // Same-day session
            (Some(open), Some(close)) => time >= open || time < close,                // Overnight session
            (Some(open), None) => time >= open,                                       // Open-ended session
            (None, Some(close)) => time < close,                                      // Close-only session
            (None, None) => false,                                                    // No session
        }
    }
}

#[derive(Clone, Debug)]
pub struct TradingHours {
    pub timezone: Tz,
    pub sunday: DaySession,
    pub monday: DaySession,
    pub tuesday: DaySession,
    pub wednesday: DaySession,
    pub thursday: DaySession,
    pub friday: DaySession,
    pub saturday: DaySession,
}

impl TradingHours {
    pub fn is_market_open(&self, current_time: DateTime<Utc>) -> bool {
        let market_time = current_time.with_timezone(&self.timezone);
        let current_time_naive = market_time.time();
        let current_weekday = market_time.weekday();

        let current_session = match current_weekday {
            Weekday::Sun => &self.sunday,
            Weekday::Mon => &self.monday,
            Weekday::Tue => &self.tuesday,
            Weekday::Wed => &self.wednesday,
            Weekday::Thu => &self.thursday,
            Weekday::Fri => &self.friday,
            Weekday::Sat => &self.saturday,
        };

        current_session.is_trading_time(current_time_naive)
    }

    pub fn seconds_until_close(&self, current_time: DateTime<Utc>) -> Option<i64> {
        let market_time = current_time.with_timezone(&self.timezone);
        let current_time_naive = market_time.time();
        let current_weekday = market_time.weekday();

        let current_session = match current_weekday {
            Weekday::Sun => &self.sunday,
            Weekday::Mon => &self.monday,
            Weekday::Tue => &self.tuesday,
            Weekday::Wed => &self.wednesday,
            Weekday::Thu => &self.thursday,
            Weekday::Fri => &self.friday,
            Weekday::Sat => &self.saturday,
        };

        match (current_session.open, current_session.close) {
            (Some(open), Some(close)) if close > open => {
                // Normal session on the same day
                if current_time_naive >= open && current_time_naive < close {
                    Some(close.num_seconds_from_midnight() as i64 - current_time_naive.num_seconds_from_midnight() as i64)
                } else {
                    None
                }
            }
            (Some(open), Some(close)) => {
                // Overnight session
                if current_time_naive >= open || current_time_naive < close {
                    let current_secs = current_time_naive.num_seconds_from_midnight() as i64;
                    let close_secs = close.num_seconds_from_midnight() as i64;
                    let until_close = if current_time_naive < close {
                        close_secs - current_secs
                    } else {
                        (86400 - current_secs) + close_secs
                    };
                    Some(until_close)
                } else {
                    None
                }
            }
            (Some(_), None) => None, // No close time for open-ended session
            (None, Some(close)) => {
                // Close-only session (for edge cases, not typically expected in trading hours)
                if current_time_naive < close {
                    Some(close.num_seconds_from_midnight() as i64 - current_time_naive.num_seconds_from_midnight() as i64)
                } else {
                    None
                }
            }
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone};
    use chrono_tz::America::Chicago;
    use crate::product_maps::rithmic::maps::CME_HOURS;

    #[test]
    fn test_sunday_monday_cycle() {
        let trading_hours = CME_HOURS;

        // Sunday open at 17:00, continues through to Monday close at 16:00
        let test_time = Chicago.with_ymd_and_hms(2024, 1, 8,9, 0, 0).unwrap().to_utc();
        assert!(trading_hours.is_market_open(test_time));
        assert_eq!(
            trading_hours.seconds_until_close(test_time),
            Some(25200), // 7 hours to Monday close at 16:00
        );

        // Monday exact close (should be closed)
        let test_time = Chicago.with_ymd_and_hms(2024, 1, 8,16, 0, 0).unwrap().to_utc();
        assert!(!trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), None);
    }

    #[test]
    fn test_friday_saturday_cycle() {
        let trading_hours = CME_HOURS;

        let test_time = Chicago.with_ymd_and_hms(2024, 1, 5,17, 0, 0).unwrap().to_utc();
        assert!(trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), Some(82800)); // Until next day’s close at 16:00

        let test_time = Chicago.with_ymd_and_hms(2024, 1, 6, 0, 0, 0).unwrap().to_utc();
        assert!(!trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), None);
    }

    #[test]
    fn test_regular_weekday_pattern() {
        let trading_hours = CME_HOURS;

        let test_time = Chicago.with_ymd_and_hms(2024, 1, 9,9, 0, 0).unwrap().to_utc();
        assert!(trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), Some(25200)); // Until 16:00

        let test_time = Chicago.with_ymd_and_hms(2024, 1, 9, 16, 0, 0).unwrap().to_utc();
        assert!(!trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), None);
    }

    #[test]
    fn test_before_open() {
        let trading_hours = CME_HOURS;

        // Before Tuesday open
        let test_time = Chicago.with_ymd_and_hms(2024, 1, 9, 16, 59, 0).unwrap().to_utc();
        assert!(!trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), None);
    }

    #[test]
    fn test_just_before_close() {
        let trading_hours = CME_HOURS;

        // Just before Monday close at 16:00
        let test_time = Chicago.with_ymd_and_hms(2024, 1, 8,15, 59, 59).unwrap().to_utc();
        assert!(trading_hours.is_market_open(test_time));
        assert_eq!(
            trading_hours.seconds_until_close(test_time),
            Some(1),
            "Expected 1 second until close, but got a different result"
        );
    }

    #[test]
    fn test_after_close() {
        let trading_hours = CME_HOURS;

        // Just after Monday close
        let test_time = Chicago.with_ymd_and_hms(2024, 1, 8, 16, 0, 1).unwrap().to_utc();
        assert!(!trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), None);
    }

    #[test]
    fn test_overnight_session_boundary() {
        let trading_hours = CME_HOURS;

        // Monday session continues from Sunday open at 17:00 through to Monday close at 16:00
        let test_time = Chicago.with_ymd_and_hms(2024, 1, 8, 1, 0, 0).unwrap().to_utc();
        assert!(trading_hours.is_market_open(test_time));
        assert_eq!(
            trading_hours.seconds_until_close(test_time),
            Some(54000), // 15 hours to close at 16:00
        );
    }

    #[test]
    fn test_unscheduled_day() {
        let trading_hours = CME_HOURS;

        // Saturday has no trading hours
        let test_time = Chicago.with_ymd_and_hms(2024, 1, 6, 12, 0, 0).unwrap().to_utc();
        assert!(!trading_hours.is_market_open(test_time));
        assert_eq!(trading_hours.seconds_until_close(test_time), None);
    }
}