use std::collections::BTreeMap;
use chrono::{DateTime, Utc, Weekday, Duration, Datelike, Timelike, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::tick::Aggressor;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::market_hours::{DaySession, TradingHours};
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;

#[derive(Debug, Clone)]
pub struct SessionTime {
    pub(crate) open: DateTime<Utc>,
    pub(crate) close: DateTime<Utc>,
    #[allow(unused)]
    pub(crate) is_same_day: bool,
}

pub struct UpdateParams {
    pub(crate) market_type: MarketType,
    pub(crate) tick_size: Decimal,
    pub(crate) decimal_accuracy: u32,
}

#[derive(Debug)]
pub enum TimeAction {
    NewSession(SessionTime),
    EnterSession(SessionTime),
    LeaveSession,
    SessionEnd(SessionTime),
}

#[derive(Debug)]
pub struct DailyConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    decimal_accuracy: u32,
    tick_size: Decimal,
    #[allow(unused)]
    last_close: Option<Price>,
    market_type: MarketType,
    trading_hours: TradingHours,
    #[allow(unused)]
    days_per_bar: i64,
    // Map of session start time to session details
    session_map: BTreeMap<DateTime<Utc>, SessionTime>,
    current_session: Option<SessionTime>,
}

impl DailyConsolidator {

    pub(crate) fn new(
        subscription: DataSubscription,
        decimal_accuracy: u32,
        tick_size: Decimal,
        trading_hours: TradingHours,
    ) -> Result<Self, FundForgeError> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "{} is an Invalid base data type for DailyConsolidator",
                subscription.base_data_type
            )));
        }

        let mut consolidator = DailyConsolidator {
            market_type: subscription.symbol.market_type.clone(),
            current_data: None,
            subscription,
            decimal_accuracy,
            tick_size,
            last_close: None,
            trading_hours: trading_hours.clone(),
            days_per_bar: 1,
            session_map: BTreeMap::new(),
            current_session: None,
        };

        // Initialize from now() but sessions will be properly extended on first update
        consolidator.initialize_session_map();
        Ok(consolidator)
    }

    fn extend_sessions(&mut self, current_time: DateTime<Utc>) {
        let tz = self.trading_hours.timezone;

        // Find the last session end time
        let last_session_end = self.session_map
            .values()
            .map(|s| s.close)
            .max()
            .unwrap_or(current_time);

        // Always try to maintain DAYS_AHEAD worth of future sessions
        let target_end = current_time + Duration::days(Self::DAYS_AHEAD);

        if target_end >= last_session_end {
            println!("Need to extend sessions. Current last end: {}, Target end: {}", last_session_end, target_end);

            // Find the start point for new sessions
            let start_date = if self.session_map.is_empty() {
                (current_time - Duration::days(Self::DAYS_TO_KEEP))
                    .with_timezone(&tz)
                    .date_naive()
            } else {
                // Start from the day after the last session
                (last_session_end + Duration::days(1))
                    .with_timezone(&tz)
                    .date_naive()
            };

            println!("Starting session extension from date: {}", start_date);

            // Generate enough days to reach our target
            let days_to_generate = (target_end - last_session_end).num_days() + 1;

            for days_offset in 0..days_to_generate {
                let current_date = start_date + Duration::days(days_offset);
                let weekday = current_date.weekday();

                println!("Generating sessions for {}: {}", weekday, current_date);

                // Handle the special Sunday->Monday case
                if weekday == Weekday::Sun {
                    // Add Sunday evening session
                    if let Some(sunday_open) = self.trading_hours.sunday.open {
                        let session_open = current_date
                            .and_hms_opt(sunday_open.hour(), sunday_open.minute(), sunday_open.second())
                            .unwrap()
                            .and_local_timezone(tz)
                            .unwrap()
                            .with_timezone(&Utc);

                        // Sunday session closes at Monday open
                        let monday_session = self.get_session_for_day(Weekday::Mon);
                        if let Some(monday_open) = monday_session.open {
                            let next_day = current_date + Duration::days(1);
                            let close_time = next_day
                                .and_hms_opt(monday_open.hour(), monday_open.minute(), monday_open.second())
                                .unwrap()
                                .and_local_timezone(tz)
                                .unwrap()
                                .with_timezone(&Utc);

                            println!("Adding Sunday session: {} -> {}", session_open, close_time);

                            self.session_map.insert(
                                session_open,
                                SessionTime {
                                    open: session_open,
                                    close: close_time,
                                    is_same_day: false,
                                },
                            );
                        }
                    }
                } else if let Some(open_time) = self.get_session_for_day(weekday).open {
                    let session_open = current_date
                        .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                        .unwrap()
                        .and_local_timezone(tz)
                        .unwrap()
                        .with_timezone(&Utc);

                    // Skip if we already have this session
                    if self.session_map.contains_key(&session_open) {
                        continue;
                    }

                    let close_time = if let Some(close_time) = self.get_session_for_day(weekday).close {
                        let close_datetime = current_date
                            .and_hms_opt(close_time.hour(), close_time.minute(), close_time.second())
                            .unwrap();
                        (close_datetime, true)
                    } else if let Some(next_open) = self.get_session_for_day(weekday.succ()).open {
                        let next_day = current_date + Duration::days(1);
                        let close_datetime = next_day
                            .and_hms_opt(next_open.hour(), next_open.minute(), next_open.second())
                            .unwrap();
                        (close_datetime, false)
                    } else {
                        continue;
                    };

                    let close_utc = close_time.0
                        .and_local_timezone(tz)
                        .unwrap()
                        .with_timezone(&Utc);

                    println!("Adding regular session: {} -> {}", session_open, close_utc);

                    self.session_map.insert(
                        session_open,
                        SessionTime {
                            open: session_open,
                            close: close_utc,
                            is_same_day: close_time.1,
                        },
                    );
                }
            }
        }

        // Clean up old sessions
        let cutoff = current_time - Duration::days(Self::DAYS_TO_KEEP);
        self.session_map.retain(|_, session| session.close >= cutoff);

        println!("\nSession map after extension:");
        for (start, session) in &self.session_map {
            println!("  {} -> {}", start, session.close);
        }
    }

    // Helper to add a session for a specific date
    fn add_session_for_date(&mut self, date: NaiveDate, tz: chrono_tz::Tz) {
        let weekday = date.weekday();
        let current_session = self.get_session_for_day(weekday);
        let next_session = self.get_session_for_day(weekday.succ());

        if let Some(open_time) = current_session.open {
            let session_open = date
                .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                .unwrap()
                .and_local_timezone(tz)
                .unwrap()
                .with_timezone(&Utc);

            // Skip if we already have this session
            if self.session_map.contains_key(&session_open) {
                return;
            }

            let session_close = if let Some(close_time) = current_session.close {
                let close_datetime = date
                    .and_hms_opt(close_time.hour(), close_time.minute(), close_time.second())
                    .unwrap();
                (close_datetime, true)
            } else if let Some(next_open) = next_session.open {
                let next_day = date + Duration::days(1);
                let close_datetime = next_day
                    .and_hms_opt(next_open.hour(), next_open.minute(), next_open.second())
                    .unwrap();
                (close_datetime, false)
            } else {
                return;
            };

            let close_utc = session_close.0
                .and_local_timezone(tz)
                .unwrap()
                .with_timezone(&Utc);

            self.session_map.insert(
                session_open,
                SessionTime {
                    open: session_open,
                    close: close_utc,
                    is_same_day: session_close.1,
                },
            );
        }
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        let time = base_data.time_utc();

        println!("Processing update for time: {}", time);

        // First check if time update would close any bars
        if let Some(closed_bar) = self.update_time(time) {
            println!("Time update closed bar at: {}", time);
            return ConsolidatedData::with_closed(base_data.clone(), closed_bar);
        }

        // Get current session without holding borrow
        let current_session = self.get_current_session(time).cloned();

        match &current_session {
            Some(session) => println!("Current session: {} -> {}", session.open, session.close),
            None => println!("No current session for time: {}", time),
        }

        match current_session {
            Some(session) if self.current_data.is_none() => {
                println!("Creating new bar for session starting at: {}", session.open);
                let new_bar = self.create_bar(base_data, session.open);
                self.current_data = Some(BaseDataEnum::Candle(new_bar.clone()));
                ConsolidatedData::with_open(BaseDataEnum::Candle(new_bar))
            }
            Some(_) => {
                if let Some(ref mut current_bar) = self.current_data {
                    let params = UpdateParams {
                        market_type: self.market_type.clone(),
                        tick_size: self.tick_size,
                        decimal_accuracy: self.decimal_accuracy,
                    };
                    Self::update_bar(&params, current_bar, base_data);
                    println!("Updated existing bar");
                }
                ConsolidatedData::with_open(base_data.clone())
            }
            None => {
                println!("No session found for time: {}", time);
                ConsolidatedData::with_open(base_data.clone())
            }
        }
    }

    fn initialize_session_map(&mut self) {
        // Start from week_start and generate for 7 days
        let tz = self.trading_hours.timezone;
        let now = Utc::now();
        let current_week_start = now
            .with_timezone(&tz)
            .date_naive()
            .week(self.trading_hours.week_start)
            .first_day();

        for days_offset in 0..7 {
            let current_date = current_week_start + Duration::days(days_offset);
            let weekday = current_date.weekday();
            let current_session = self.get_session_for_day(weekday);
            let next_session = self.get_session_for_day(weekday.succ());

            if let Some(open_time) = current_session.open {
                // Calculate session open in UTC
                let session_open = current_date
                    .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                    .unwrap()
                    .and_local_timezone(tz)
                    .unwrap()
                    .with_timezone(&Utc);

                // Calculate session close
                let session_close = if let Some(close_time) = current_session.close {
                    // Same day close
                    let close_datetime = current_date
                        .and_hms_opt(close_time.hour(), close_time.minute(), close_time.second())
                        .unwrap();
                    (close_datetime, true)
                } else if let Some(next_open) = next_session.open {
                    // Next day's open is our close
                    let next_day = current_date + Duration::days(1);
                    let close_datetime = next_day
                        .and_hms_opt(next_open.hour(), next_open.minute(), next_open.second())
                        .unwrap();
                    (close_datetime, false)
                } else {
                    continue; // Skip if we can't determine close time
                };

                let close_utc = session_close.0
                    .and_local_timezone(tz)
                    .unwrap()
                    .with_timezone(&Utc);

                self.session_map.insert(
                    session_open,
                    SessionTime {
                        open: session_open,
                        close: close_utc,
                        is_same_day: session_close.1,
                    },
                );
            }
        }
    }

    fn get_session_for_day(&self, weekday: Weekday) -> &DaySession {
        match weekday {
            Weekday::Mon => &self.trading_hours.monday,
            Weekday::Tue => &self.trading_hours.tuesday,
            Weekday::Wed => &self.trading_hours.wednesday,
            Weekday::Thu => &self.trading_hours.thursday,
            Weekday::Fri => &self.trading_hours.friday,
            Weekday::Sat => &self.trading_hours.saturday,
            Weekday::Sun => &self.trading_hours.sunday,
        }
    }

    fn get_current_session(&self, time: DateTime<Utc>) -> Option<&SessionTime> {
        self.session_map
            .range(..=time)
            .next_back()
            .map(|(_, session)| session)
            .filter(|session| time < session.close)
    }

    fn is_session_end(&self, time: DateTime<Utc>) -> bool {
        if let Some(session) = self.get_current_session(time) {
            time >= session.close
        } else {
            true
        }
    }

    fn update_bar(
        params: &UpdateParams,
        current_bar: &mut BaseDataEnum,
        new_data: &BaseDataEnum
    ) {
        match (current_bar, new_data) {
            (BaseDataEnum::Candle(candle), BaseDataEnum::Tick(tick)) => {
                candle.high = candle.high.max(tick.price);
                candle.low = candle.low.min(tick.price);
                candle.close = tick.price;
                candle.range = params.market_type.round_price(
                    candle.high - candle.low,
                    params.tick_size,
                    params.decimal_accuracy,
                );

                match tick.aggressor {
                    Aggressor::Buy => candle.bid_volume += tick.volume,
                    Aggressor::Sell => candle.ask_volume += tick.volume,
                    _ => {}
                }

                candle.volume += tick.volume;
            }
            (BaseDataEnum::Candle(current), BaseDataEnum::Candle(new)) => {
                current.high = current.high.max(new.high);
                current.low = current.low.min(new.low);
                current.close = new.close;
                current.range = params.market_type.round_price(
                    current.high - current.low,
                    params.tick_size,
                    params.decimal_accuracy,
                );
                current.volume += new.volume;
                current.ask_volume += new.ask_volume;
                current.bid_volume += new.bid_volume;
            }
            _ => panic!("Invalid base data type combination for update"),
        }
    }

    // Helper method to check if we've moved to a new session
    fn is_new_session(&self, time: DateTime<Utc>) -> bool {
        if let Some(current_session) = &self.current_session {
            if let Some(new_session) = self.get_current_session(time) {
                return new_session.open != current_session.open;
            }
        }
        false
    }

    // Helper method to check if we're at a session boundary
    fn is_session_boundary(&self, time: DateTime<Utc>) -> bool {
        if let Some(session) = &self.current_session {
            time >= session.close
        } else {
            false
        }
    }

    fn create_bar(&self, base_data: &BaseDataEnum, session_open: DateTime<Utc>) -> Candle {
        match base_data {
            BaseDataEnum::Tick(tick) => {
                let (ask_volume, bid_volume) = match tick.aggressor {
                    Aggressor::Buy => (dec!(0.0), tick.volume),
                    Aggressor::Sell => (tick.volume, dec!(0.0)),
                    Aggressor::None => (dec!(0), dec!(0)),
                };

                Candle::new(
                    self.subscription.symbol.clone(),
                    tick.price,
                    tick.volume,
                    ask_volume,
                    bid_volume,
                    session_open.to_string(),
                    self.subscription.resolution.clone(),
                    self.subscription.candle_type.clone().unwrap(),
                )
            }
            BaseDataEnum::Candle(candle) => {
                let mut new_candle = candle.clone();
                new_candle.is_closed = false;
                new_candle.resolution = self.subscription.resolution.clone();
                new_candle.time = session_open.to_string();
                new_candle
            }
            _ => panic!("Invalid base data type for Candle consolidator"),
        }
    }

    // Add these constants for session management
    const DAYS_TO_KEEP: i64 = 7; // Keep a week of historical sessions
    const DAYS_AHEAD: i64 = 7;   // Generate a week of future sessions

    // Modify update_time to handle session extension
    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        // Extend sessions if needed before processing the update
        self.extend_sessions(time);

        let action = {
            let current_session_info = self.get_current_session(time);
            let current_stored_session = self.current_session.as_ref();

            match (current_session_info, current_stored_session) {
                (Some(new_session), Some(stored_session))
                if new_session.open != stored_session.open => {
                    Some(TimeAction::NewSession(new_session.clone()))
                }
                (Some(new_session), None) => {
                    Some(TimeAction::EnterSession(new_session.clone()))
                }
                (None, Some(_)) => {
                    Some(TimeAction::LeaveSession)
                }
                (Some(session), Some(_)) if time >= session.close => {
                    Some(TimeAction::SessionEnd(session.clone()))
                }
                _ => None
            }
        };

        // Rest of the update_time implementation remains the same
        match action {
            Some(TimeAction::NewSession(new_session)) => {
                let closed_bar = self.current_data.take().map(|mut bar| {
                    bar.set_is_closed(true);
                    bar
                });
                self.current_session = Some(new_session);
                closed_bar
            }
            Some(TimeAction::EnterSession(new_session)) => {
                self.current_session = Some(new_session);
                None
            }
            Some(TimeAction::LeaveSession) => {
                let closed_bar = self.current_data.take().map(|mut bar| {
                    bar.set_is_closed(true);
                    bar
                });
                self.current_session = None;
                closed_bar
            }
            Some(TimeAction::SessionEnd(new_session)) => {
                let closed_bar = self.current_data.take().map(|mut bar| {
                    bar.set_is_closed(true);
                    bar
                });
                self.current_session = Some(new_session);
                closed_bar
            }
            None => None
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;
    use super::*;
    use chrono_tz::America::New_York;
    use crate::standardized_types::datavendor_enum::DataVendor;
    use crate::standardized_types::resolution::Resolution;
    use crate::standardized_types::subscriptions::{CandleType, Symbol};

    fn setup_trading_hours() -> TradingHours {
        TradingHours {
            timezone: New_York,
            sunday: DaySession {
                open: Some(NaiveTime::from_hms_opt(17, 0, 0).unwrap()),
                close: None
            },
            monday: DaySession {
                open: Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
                close: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
            },
            tuesday: DaySession {
                open: Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
                close: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
            },
            wednesday: DaySession {
                open: Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
                close: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
            },
            thursday: DaySession {
                open: Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
                close: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
            },
            friday: DaySession {
                open: Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
                close: Some(NaiveTime::from_hms_opt(16, 0, 0).unwrap()),
            },
            saturday: DaySession { open: None, close: None },
            week_start: Weekday::Sun,
        }
    }

    #[tokio::test]
    async fn test_real_time_session_management() {
        let trading_hours = setup_trading_hours();
        let subscription = DataSubscription {
            symbol: Symbol::new("TEST".to_string(), DataVendor::Test, MarketType::CFD),
            base_data_type: BaseDataType::Candles,
            resolution: Resolution::Day,
            candle_type: Some(CandleType::CandleStick),
            market_type: MarketType::CFD,
        };

        let mut consolidator = DailyConsolidator::new(
            subscription,
            2,
            dec!(0.01),
            trading_hours,
        ).unwrap();

        // 1. Test initial session map creation
        let now = Utc::now();
        let initial_session_count = consolidator.session_map.len();
        assert!(initial_session_count > 0, "Should create initial sessions");

        // 2. Test session extension
        let future_time = now + Duration::days(5);
        consolidator.update_time(future_time);
        assert!(
            consolidator.session_map.len() > initial_session_count,
            "Should extend sessions when approaching end"
        );

        // 3. Test session cleanup
        let past_time = now - Duration::days(10);
        let candle = Candle::new(
            consolidator.subscription.symbol.clone(),
            dec!(100),
            dec!(1000),
            dec!(500),
            dec!(500),
            past_time.to_string(),
            Resolution::Day,
            CandleType::CandleStick,
        );

        consolidator.update(&BaseDataEnum::Candle(candle));

        // Verify old sessions are cleaned up
        let oldest_session = consolidator.session_map.first_key_value().unwrap().1.open;
        assert!(
            oldest_session > now - Duration::days(DailyConsolidator::DAYS_TO_KEEP + 1),
            "Should clean up sessions older than DAYS_TO_KEEP"
        );

        // 4. Test continuous session extension
        let mut test_time = now;
        let mut session_times = Vec::new();

        // Simulate advancing time over multiple days
        for day in 0..10 {
            test_time = now + Duration::days(day);
            consolidator.update_time(test_time);

            if let Some(session) = consolidator.get_current_session(test_time) {
                session_times.push((session.open, session.close));
            }

            // Verify we maintain a rolling window of sessions
            let session_span = consolidator.session_map.iter()
                .map(|(_, s)| s.close - s.open)
                .sum::<Duration>();

            assert!(
                session_span.num_days() <= DailyConsolidator::DAYS_AHEAD + DailyConsolidator::DAYS_TO_KEEP,
                "Should maintain proper session window"
            );
        }

        // 5. Verify session continuity
        for times in session_times.windows(2) {
            if let [(_, prev_close), (next_open, _)] = times {
                // For continuous markets, next session should start when previous ends
                assert!(
                    *next_open >= *prev_close,
                    "Sessions should not overlap: prev_close={}, next_open={}",
                    prev_close,
                    next_open
                );
            }
        }

        // 6. Test real-time consolidation with session boundaries
        let mut daily_bars = Vec::new();
        let mut test_time = now;

        // Generate test data spanning multiple sessions
        for hour in 0..48 {
            test_time = now + Duration::hours(hour);

            let candle = Candle::new(
                consolidator.subscription.symbol.clone(),
                dec!(100),
                dec!(1000),
                dec!(500),
                dec!(500),
                test_time.to_string(),
                Resolution::Day,
                CandleType::CandleStick,
            );

            // Process updates
            if let Some(closed_bar) = consolidator.update_time(test_time) {
                daily_bars.push(closed_bar);
            }

            let result = consolidator.update(&BaseDataEnum::Candle(candle));
            if let Some(closed_bar) = result.closed_data {
                daily_bars.push(closed_bar);
            }
        }

        // Verify bars close at session boundaries
        for bar in daily_bars {
            if let BaseDataEnum::Candle(candle) = bar {
                let bar_time = DateTime::parse_from_rfc3339(&candle.time)
                    .unwrap()
                    .with_timezone(&Utc);

                // Find the session this bar belongs to
                let session = consolidator.session_map
                    .range(..=bar_time)
                    .next_back()
                    .map(|(_, s)| s);

                assert!(
                    session.is_some(),
                    "Closed bar should correspond to a valid session"
                );

                if let Some(session) = session {
                    assert!(
                        bar_time <= session.close,
                        "Bar should close at or before session end"
                    );
                }
            }
        }
    }
}
