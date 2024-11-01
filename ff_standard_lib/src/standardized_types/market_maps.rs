use chrono::NaiveTime;
use chrono_tz::Tz;

pub mod product_trading_hours;

#[derive(Debug, Clone)]
pub struct DaySession {
    pub open: Option<NaiveTime>,
    pub close: Option<NaiveTime>,
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

impl DaySession {
    pub fn is_trading_time(&self, time: NaiveTime) -> bool {
        match (self.open, self.close) {
            (Some(open), Some(close)) => {
                if close > open {
                    // Normal session (e.g., 8:00 to 16:00)
                    time >= open && time < close
                } else {
                    // Overnight session (e.g., 17:00 to 16:00 next day)
                    time >= open || time < close
                }
            },
            (Some(open), None) => {
                // Session that starts but doesn't end on this day
                time >= open
            },
            (None, Some(close)) => {
                // Session that ends but didn't start on this day
                time < close
            },
            (None, None) => false,
        }
    }
}
