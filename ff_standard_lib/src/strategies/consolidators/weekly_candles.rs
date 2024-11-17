use chrono::{DateTime, Utc, Weekday, Duration, Datelike, Timelike, NaiveTime};
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::market_hours::{TradingHours};

struct WeeklyConsolidator {
    trading_hours: TradingHours,
    week_start_day: Weekday,
    week_start_session: Option<(Weekday, NaiveTime)>,  // First trading session of week
    week_end_session: Option<(Weekday, NaiveTime)>,    // Last trading session of week
}

#[allow(dead_code)]
impl WeeklyConsolidator {
    pub(crate) async fn new(
        trading_hours: TradingHours,
        week_start_day: Weekday,
    ) -> Result<Self, FundForgeError> {
        // Calculate the week schedule
        let (week_start_session, week_end_session) = Self::calculate_week_schedule(&trading_hours, week_start_day);

        Ok(WeeklyConsolidator {
            trading_hours,
            week_start_day,
            week_start_session,
            week_end_session,
        })
    }

    fn calculate_week_schedule(trading_hours: &TradingHours, week_start: Weekday)
                               -> (Option<(Weekday, NaiveTime)>, Option<(Weekday, NaiveTime)>) {
        let weekdays = [
            week_start,
            week_start.succ(),
            week_start.succ().succ(),
            week_start.succ().succ().succ(),
            week_start.succ().succ().succ().succ(),
            week_start.succ().succ().succ().succ().succ(),
            week_start.succ().succ().succ().succ().succ().succ(),
        ];

        let mut first_session = None;
        let mut last_session = None;

        // Find first trading session
        for &day in weekdays.iter() {
            let session = match day {
                Weekday::Mon => &trading_hours.monday,
                Weekday::Tue => &trading_hours.tuesday,
                Weekday::Wed => &trading_hours.wednesday,
                Weekday::Thu => &trading_hours.thursday,
                Weekday::Fri => &trading_hours.friday,
                Weekday::Sat => &trading_hours.saturday,
                Weekday::Sun => &trading_hours.sunday,
            };

            if let Some(open) = session.open {
                first_session = Some((day, open));
                break;
            }
        }

        // Find last trading session
        for &day in weekdays.iter().rev() {
            let session = match day {
                Weekday::Mon => &trading_hours.monday,
                Weekday::Tue => &trading_hours.tuesday,
                Weekday::Wed => &trading_hours.wednesday,
                Weekday::Thu => &trading_hours.thursday,
                Weekday::Fri => &trading_hours.friday,
                Weekday::Sat => &trading_hours.saturday,
                Weekday::Sun => &trading_hours.sunday,
            };

            if let Some(close) = session.close {
                last_session = Some((day, close));
                break;
            }
        }

        (first_session, last_session)
    }

    fn is_week_end(&self, time: DateTime<Utc>) -> bool {
        let market_time = time.with_timezone(&self.trading_hours.timezone);

        if let Some((end_day, end_time)) = self.week_end_session {
            market_time.weekday() == end_day && market_time.time() >= end_time
        } else {
            false
        }
    }

    fn get_week_start(&self, time: DateTime<Utc>) -> DateTime<Utc> {
        let market_time = time.with_timezone(&self.trading_hours.timezone);
        let mut current_day = market_time;

        // Go back to week start day
        while current_day.weekday() != self.week_start_day {
            current_day = current_day - Duration::days(1);
        }

        // If we have a defined first session, use its time
        if let Some((start_day, start_time)) = self.week_start_session {
            while current_day.weekday() != start_day {
                current_day = current_day + Duration::days(1);
            }

            current_day.date_naive()
                .and_hms_opt(start_time.hour(), start_time.minute(), start_time.second())
                .unwrap()
                .and_local_timezone(self.trading_hours.timezone)
                .unwrap()
                .with_timezone(&Utc)
        } else {
            current_day.with_timezone(&Utc)
        }
    }
}