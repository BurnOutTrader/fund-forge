use chrono::NaiveTime;
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct DaySession {
    pub open: Option<NaiveTime>,
    pub close: Option<NaiveTime>,
}

#[derive(Debug, Clone)]
pub struct TradingHours {
    pub timezone: Tz,
    pub sunday: DaySession,
    pub monday: DaySession,
    pub tuesday: DaySession,
    pub wednesday: DaySession,
    pub thursday: DaySession,
    pub friday: DaySession,
}
