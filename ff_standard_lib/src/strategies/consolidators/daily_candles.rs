use std::collections::BTreeMap;
use chrono::{DateTime, Utc, Weekday, TimeZone, Duration, Datelike, Timelike};
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
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;

#[derive(Debug, Clone)]
pub struct SessionTime {
    pub(crate) open: DateTime<Utc>,
    pub(crate) close: DateTime<Utc>,
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
    last_close: Option<Price>,
    market_type: MarketType,
    trading_hours: TradingHours,
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

        consolidator.initialize_session_map();
        Ok(consolidator)
    }

    #[cfg(test)]
    pub(crate) async fn new_with_reference_time(
        subscription: DataSubscription,
        decimal_accuracy: u32,
        tick_size: Decimal,
        trading_hours: TradingHours,
        reference_time: DateTime<Utc>,
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

        consolidator.initialize_session_map_from(reference_time);
        Ok(consolidator)
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

    fn initialize_session_map_from(&mut self, reference_time: DateTime<Utc>) {
        let tz = self.trading_hours.timezone;
        // Find the number of weeks needed to cover the test period (2 weeks should be enough)
        let weeks_to_generate = 2;

        let current_week_start = reference_time
            .with_timezone(&tz)
            .date_naive()
            .week(self.trading_hours.week_start)
            .first_day();

        println!("Reference time: {}", reference_time);
        println!("First week start: {}", current_week_start);
        println!("Trading hours timezone: {}", tz);
        println!("Generating {} weeks of sessions", weeks_to_generate);

        // Generate sessions for multiple weeks
        for week_offset in 0..weeks_to_generate {
            let week_start = current_week_start + Duration::weeks(week_offset);
            println!("\nGenerating sessions for week starting: {}", week_start);

            for days_offset in 0..7 {
                let current_date = week_start + Duration::days(days_offset);
                let weekday = current_date.weekday();
                println!("\nProcessing weekday: {}", weekday);

                let current_session = self.get_session_for_day(weekday);
                let next_session = self.get_session_for_day(weekday.succ());

                println!("Current session open: {:?}, close: {:?}",
                         current_session.open,
                         current_session.close);

                if let Some(open_time) = current_session.open {
                    let session_open = current_date
                        .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                        .unwrap()
                        .and_local_timezone(tz)
                        .unwrap()
                        .with_timezone(&Utc);

                    let session_close = if let Some(close_time) = current_session.close {
                        let close_datetime = current_date
                            .and_hms_opt(close_time.hour(), close_time.minute(), close_time.second())
                            .unwrap();
                        println!("Same day close time: {}", close_datetime);
                        (close_datetime, true)
                    } else if let Some(next_open) = next_session.open {
                        let next_day = current_date + Duration::days(1);
                        let close_datetime = next_day
                            .and_hms_opt(next_open.hour(), next_open.minute(), next_open.second())
                            .unwrap();
                        println!("Next day close time: {}", close_datetime);
                        (close_datetime, false)
                    } else {
                        println!("Skipping - no close time found");
                        continue;
                    };

                    let close_utc = session_close.0
                        .and_local_timezone(tz)
                        .unwrap()
                        .with_timezone(&Utc);

                    println!("Adding session: {} -> {} (UTC)", session_open, close_utc);

                    self.session_map.insert(
                        session_open,
                        SessionTime {
                            open: session_open,
                            close: close_utc,
                            is_same_day: session_close.1,
                        },
                    );
                } else {
                    println!("No opening time for this day");
                }
            }
        }

        println!("\nFinal session map:");
        for (start, session) in &self.session_map {
            println!("Start: {}, Close: {}, Same day: {}",
                     start, session.close, session.is_same_day);
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

    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        // Get ALL the information we need upfront before any mutations
        let action = {
            let current_session_info = self.get_current_session(time);
            let current_stored_session = self.current_session.as_ref();

            match (current_session_info, current_stored_session) {
                // Moving to a new session
                (Some(new_session), Some(stored_session))
                if new_session.open != stored_session.open => {
                    Some(TimeAction::NewSession(new_session.clone()))
                }
                // Entering a session
                (Some(new_session), None) => {
                    Some(TimeAction::EnterSession(new_session.clone()))
                }
                // Leaving trading hours
                (None, Some(_)) => {
                    Some(TimeAction::LeaveSession)
                }
                // At session boundary
                (Some(session), Some(_)) if time >= session.close => {
                    Some(TimeAction::SessionEnd(session.clone()))
                }
                // No action needed
                _ => None
            }
        };

        // Now handle the action with no active borrows
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

    pub fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        let time = base_data.time_utc();

        // First check if time update would close any bars
        if let Some(closed_bar) = self.update_time(time) {
            return ConsolidatedData::with_closed(base_data.clone(), closed_bar);
        }

        // Get current session without holding borrow
        let current_session = self.get_current_session(time).cloned();

        match current_session {
            Some(session) if self.current_data.is_none() => {
                // Start new bar
                let new_bar = self.create_bar(base_data, session.open);
                self.current_data = Some(BaseDataEnum::Candle(new_bar.clone()));
                ConsolidatedData::with_open(BaseDataEnum::Candle(new_bar))
            }
            Some(_) => {
                // Update existing bar
                if let Some(ref mut current_bar) = self.current_data {
                    let params = UpdateParams {
                        market_type: self.market_type.clone(),
                        tick_size: self.tick_size,
                        decimal_accuracy: self.decimal_accuracy,
                    };
                    Self::update_bar(&params, current_bar, base_data);
                }
                ConsolidatedData::with_open(base_data.clone())
            }
            None => ConsolidatedData::with_open(base_data.clone())
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
}
#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, NaiveTime, DateTime, Utc};
    use super::*;
    use chrono_tz::America::New_York;
    use crate::standardized_types::base_data::candle::generate_5_day_candle_data;
    use crate::standardized_types::datavendor_enum::DataVendor;
    use crate::standardized_types::subscriptions::Symbol;

    // Helper function to parse DateTime<Utc> strings
    fn parse_datetime(datetime_str: &str) -> DateTime<Utc> {
        // Remove " UTC" suffix if present
        let cleaned_str = datetime_str.trim_end_matches(" UTC");

        DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDateTime::parse_from_str(cleaned_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|e| panic!("Failed to parse datetime '{}': {}", datetime_str, e)),
            Utc,
        )
    }

    fn setup_trading_hours() -> TradingHours {
        // Trading hours setup remains the same
        TradingHours {
            timezone: New_York,
            sunday: DaySession {
                open: Some(NaiveTime::from_hms_opt(17, 0, 0).unwrap()),
                close: None // Closes at Monday open
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
    async fn test_daily_consolidation() {
        let trading_hours = setup_trading_hours();
        let subscription = DataSubscription {
            symbol: Symbol::new("TEST".to_string(), DataVendor::Test, MarketType::CFD),
            base_data_type: BaseDataType::Candles,
            resolution: Resolution::Day,
            candle_type: Some(CandleType::CandleStick),
            market_type: MarketType::CFD,
        };

        // Get the start time from the test data and print it
        let test_data = generate_5_day_candle_data();
        let first_time = parse_datetime(&test_data.first().unwrap().time);
        let last_time = parse_datetime(&test_data.last().unwrap().time);

        println!("\nTest data range:");
        println!("First candle time: {}", first_time);
        println!("Last candle time: {}", last_time);

        let mut consolidator = DailyConsolidator::new_with_reference_time(
            subscription,
            2,
            dec!(0.01),
            trading_hours,
            first_time,
        ).await.unwrap();

        // Print session map after initialization
        println!("\nInitial session map:");
        for (start, session) in consolidator.session_map.iter() {
            println!("Session: {} -> {}", start, session.close);
        }

        let mut daily_bars = Vec::new();

        for (i, candle) in test_data.iter().enumerate() {
            let time = parse_datetime(&candle.time);

            if i % 24 == 0 {
                println!("\nProcessing day {} starting at {}", i / 24, time);
                if let Some(session) = consolidator.get_current_session(time) {
                    println!("Current session: Open={}, Close={}", session.open, session.close);
                } else {
                    println!("No active session");
                }
            }

            if let Some(closed_bar) = consolidator.update_time(time) {
                println!("Time update closed bar at {}", time);
                daily_bars.push(closed_bar);
            }

            let result = consolidator.update(&BaseDataEnum::Candle(candle.clone()));
            if let Some(closed_bar) = result.closed_data {
                println!("Data update closed bar at {}", time);
                daily_bars.push(closed_bar);
            }

            if let Some(ref current_bar) = consolidator.current_data {
                if i % 24 == 0 {
                    println!("Current bar: {:?}", current_bar);
                }
            }
        }

        println!("\nGenerated {} daily bars:", daily_bars.len());
        for (i, bar) in daily_bars.iter().enumerate() {
            if let BaseDataEnum::Candle(candle) = bar {
                println!("Bar {}: Time={}, Open={}, High={}, Low={}, Close={}",
                         i, candle.time, candle.open, candle.high, candle.low, candle.close);
            }
        }

        assert!(!daily_bars.is_empty(), "Should produce at least one bar");

        let market_data: Vec<&Candle> = test_data.iter()
            .filter(|c| {
                let time = parse_datetime(&c.time);
                consolidator.get_current_session(time).is_some()
            })
            .collect();

        println!("\nFound {} candles during market hours", market_data.len());

        for (i, bar) in daily_bars.iter().enumerate() {
            if let BaseDataEnum::Candle(candle) = bar {
                println!("Verifying bar {}: {}", i, candle.time);

                let bar_time = parse_datetime(&candle.time);

                if let Some(session) = consolidator.get_current_session(bar_time) {
                    let session_data: Vec<&Candle> = market_data.iter()
                        .filter(|c| {
                            let time = parse_datetime(&c.time);
                            time >= session.open && time < session.close
                        })
                        .copied()
                        .collect();

                    if !session_data.is_empty() {
                        let expected_high = session_data.iter().map(|c| c.high).max().unwrap();
                        let expected_low = session_data.iter().map(|c| c.low).min().unwrap();
                        let expected_volume: Decimal = session_data.iter().map(|c| c.volume).sum();

                        assert_eq!(candle.high, expected_high, "Bar {} high mismatch", i);
                        assert_eq!(candle.low, expected_low, "Bar {} low mismatch", i);
                        assert_eq!(candle.volume, expected_volume, "Bar {} volume mismatch", i);
                    }
                }
            }
        }
    }
}

