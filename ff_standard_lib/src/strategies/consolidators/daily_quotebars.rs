use std::collections::BTreeMap;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc, Weekday};
use rust_decimal::Decimal;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::market_hours::{DaySession, TradingHours};
use crate::standardized_types::new_types::Price;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;
use crate::strategies::consolidators::daily_candles::{SessionTime, TimeAction, UpdateParams};

#[derive(Debug, Clone)]
pub struct DailyQuoteConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    decimal_accuracy: u32,
    tick_size: Decimal,
    #[allow(unused)]
    last_ask_close: Option<Price>,
    #[allow(unused)]
    last_bid_close: Option<Price>,
    market_type: MarketType,
    trading_hours: TradingHours,
    #[allow(unused)]
    days_per_bar: i64,
    session_map: BTreeMap<DateTime<Utc>, SessionTime>,
    current_session: Option<SessionTime>,
}

impl DailyQuoteConsolidator {
    pub(crate) fn new(
        subscription: DataSubscription,
        decimal_accuracy: u32,
        tick_size: Decimal,
        trading_hours: TradingHours,
    ) -> Result<Self, FundForgeError> {
        if subscription.base_data_type != BaseDataType::Quotes
            && subscription.base_data_type != BaseDataType::QuoteBars {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "{} is an Invalid base data type for DailyQuoteConsolidator",
                subscription.base_data_type
            )));
        }

        let mut consolidator = DailyQuoteConsolidator {
            market_type: subscription.symbol.market_type.clone(),
            current_data: None,
            subscription,
            decimal_accuracy,
            tick_size,
            last_ask_close: None,
            last_bid_close: None,
            trading_hours: trading_hours.clone(),
            days_per_bar: 1,
            session_map: BTreeMap::new(),
            current_session: None,
        };

        consolidator.initialize_session_map();
        Ok(consolidator)
    }

    fn extend_sessions(&mut self, current_time: DateTime<Utc>) {
        let tz = self.trading_hours.timezone;

        // Get the earliest time we need to have sessions for
        let earliest_needed = current_time - Duration::days(Self::DAYS_TO_KEEP);

        // If we have no sessions or our earliest session is too late, reinitialize
        if self.session_map.is_empty() ||
            self.session_map.first_key_value().map(|(t, _)| *t).unwrap_or(current_time) > earliest_needed {
            println!("Reinitializing quote session map for time: {}", current_time);

            // Start from earliest needed time
            let start_week = earliest_needed
                .with_timezone(&tz)
                .date_naive()
                .week(self.trading_hours.week_start)
                .first_day();

            // Generate enough weeks to cover our window plus future sessions
            let total_days = Self::DAYS_TO_KEEP + Self::DAYS_AHEAD;

            for days_offset in 0..total_days {
                let current_date = start_week + Duration::days(days_offset);
                self.add_session_for_date(current_date, tz);
            }
        } else {
            // Extend forward if needed
            let last_session_end = self.session_map
                .values()
                .map(|s| s.close)
                .max()
                .unwrap_or(current_time);

            if current_time + Duration::days(1) >= last_session_end {
                println!("Extending quote sessions forward from: {}", last_session_end);

                let extension_start = last_session_end
                    .with_timezone(&tz)
                    .date_naive();

                for days_offset in 0..Self::DAYS_AHEAD {
                    let current_date = extension_start + Duration::days(days_offset);
                    self.add_session_for_date(current_date, tz);
                }
            }
        }

        // Clean up old sessions
        let cutoff = current_time - Duration::days(Self::DAYS_TO_KEEP);
        self.session_map.retain(|_, session| session.close >= cutoff);

        println!("Quote session map after extension:");
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

            // Handle special case for Sunday -> Monday
            let is_sunday_evening = weekday == Weekday::Sun;

            self.session_map.insert(
                session_open,
                SessionTime {
                    open: session_open,
                    close: close_utc,
                    is_same_day: !is_sunday_evening && session_close.1,
                },
            );
        }
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        let time = base_data.time_utc();

        println!("Processing quote update for time: {}", time);

        // First check if time update would close any bars
        if let Some(closed_bar) = self.update_time(time) {
            println!("Time update closed quote bar at: {}", time);
            return ConsolidatedData::with_closed(base_data.clone(), closed_bar);
        }

        // Get current session without holding borrow
        let current_session = self.get_current_session(time).cloned();

        match &current_session {
            Some(session) => println!("Current quote session: {} -> {}", session.open, session.close),
            None => println!("No current quote session for time: {}", time),
        }

        match current_session {
            Some(session) if self.current_data.is_none() => {
                println!("Creating new quote bar for session starting at: {}", session.open);
                let new_bar = self.create_bar(base_data, session.open);
                self.current_data = Some(BaseDataEnum::QuoteBar(new_bar.clone()));
                ConsolidatedData::with_open(BaseDataEnum::QuoteBar(new_bar))
            }
            Some(_) => {
                if let Some(ref mut current_bar) = self.current_data {
                    let params = UpdateParams {
                        market_type: self.market_type.clone(),
                        tick_size: self.tick_size,
                        decimal_accuracy: self.decimal_accuracy,
                    };
                    Self::update_bar(&params, current_bar, base_data);
                    println!("Updated existing quote bar");
                }
                ConsolidatedData::with_open(base_data.clone())
            }
            None => {
                println!("No quote session found for time: {}", time);
                ConsolidatedData::with_open(base_data.clone())
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_reference_time(
        subscription: DataSubscription,
        decimal_accuracy: u32,
        tick_size: Decimal,
        trading_hours: TradingHours,
        reference_time: DateTime<Utc>,
    ) -> Result<Self, FundForgeError> {
        if subscription.base_data_type != BaseDataType::Quotes
            && subscription.base_data_type != BaseDataType::QuoteBars {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "{} is an Invalid base data type for DailyQuoteConsolidator",
                subscription.base_data_type
            )));
        }

        let mut consolidator = DailyQuoteConsolidator {
            market_type: subscription.symbol.market_type.clone(),
            current_data: None,
            subscription,
            decimal_accuracy,
            tick_size,
            last_ask_close: None,
            last_bid_close: None,
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
                    (close_datetime, true)
                } else if let Some(next_open) = next_session.open {
                    let next_day = current_date + Duration::days(1);
                    let close_datetime = next_day
                        .and_hms_opt(next_open.hour(), next_open.minute(), next_open.second())
                        .unwrap();
                    (close_datetime, false)
                } else {
                    continue;
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

    #[allow(dead_code)]
    fn initialize_session_map_from(&mut self, reference_time: DateTime<Utc>) {
        let tz = self.trading_hours.timezone;
        let weeks_to_generate = 2;

        let current_week_start = reference_time
            .with_timezone(&tz)
            .date_naive()
            .week(self.trading_hours.week_start)
            .first_day();

        for week_offset in 0..weeks_to_generate {
            let week_start = current_week_start + Duration::weeks(week_offset);
            for days_offset in 0..7 {
                let current_date = week_start + Duration::days(days_offset);
                let weekday = current_date.weekday();
                let current_session = self.get_session_for_day(weekday);
                let next_session = self.get_session_for_day(weekday.succ());

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
                        (close_datetime, true)
                    } else if let Some(next_open) = next_session.open {
                        let next_day = current_date + Duration::days(1);
                        let close_datetime = next_day
                            .and_hms_opt(next_open.hour(), next_open.minute(), next_open.second())
                            .unwrap();
                        (close_datetime, false)
                    } else {
                        continue;
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

    fn update_bar(
        params: &UpdateParams,
        current_bar: &mut BaseDataEnum,
        new_data: &BaseDataEnum
    ) {
        match (current_bar, new_data) {
            (BaseDataEnum::QuoteBar(quote_bar), BaseDataEnum::Quote(quote)) => {
                quote_bar.ask_high = quote_bar.ask_high.max(quote.ask);
                quote_bar.ask_low = quote_bar.ask_low.min(quote.ask);
                quote_bar.bid_high = quote_bar.bid_high.max(quote.bid);
                quote_bar.bid_low = quote_bar.bid_low.min(quote.bid);
                quote_bar.ask_close = quote.ask;
                quote_bar.bid_close = quote.bid;
                quote_bar.volume += quote.ask_volume + quote.bid_volume;
                quote_bar.ask_volume += quote.ask_volume;
                quote_bar.bid_volume += quote.bid_volume;
                quote_bar.range = params.market_type.round_price(
                    quote_bar.ask_high - quote_bar.bid_low,
                    params.tick_size,
                    params.decimal_accuracy,
                );
                quote_bar.spread = params.market_type.round_price(
                    quote_bar.ask_close - quote_bar.bid_close,
                    params.tick_size,
                    params.decimal_accuracy,
                );
            }
            (BaseDataEnum::QuoteBar(current), BaseDataEnum::QuoteBar(new)) => {
                current.ask_high = current.ask_high.max(new.ask_high);
                current.ask_low = current.ask_low.min(new.ask_low);
                current.bid_high = current.bid_high.max(new.bid_high);
                current.bid_low = current.bid_low.min(new.bid_low);
                current.ask_close = new.ask_close;
                current.bid_close = new.bid_close;
                current.volume += new.volume;
                current.ask_volume += new.ask_volume;
                current.bid_volume += new.bid_volume;
                current.range = params.market_type.round_price(
                    current.ask_high - current.bid_low,
                    params.tick_size,
                    params.decimal_accuracy,
                );
                current.spread = params.market_type.round_price(
                    current.ask_close - current.bid_close,
                    params.tick_size,
                    params.decimal_accuracy,
                );
            }
            _ => panic!("Invalid base data type combination for update"),
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

    fn create_bar(&self, base_data: &BaseDataEnum, session_open: DateTime<Utc>) -> QuoteBar {
        match base_data {
            BaseDataEnum::Quote(quote) => {
                QuoteBar::new(
                    self.subscription.symbol.clone(),
                    quote.ask,
                    quote.bid,
                    quote.ask_volume,
                    quote.bid_volume,
                    quote.bid_volume + quote.ask_volume,
                    session_open.to_string(),
                    self.subscription.resolution.clone(),
                    CandleType::CandleStick
                )
            }
            BaseDataEnum::QuoteBar(quote_bar) => {
                let mut new_quote_bar = quote_bar.clone();
                new_quote_bar.is_closed = false;
                new_quote_bar.resolution = self.subscription.resolution.clone();
                new_quote_bar.time = session_open.to_string();
                new_quote_bar
            }
            _ => panic!("Invalid base data type for QuoteBar consolidator"),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, NaiveTime, DateTime, Utc};
    use super::*;
    use chrono_tz::America::New_York;
    use rust_decimal_macros::dec;
    use crate::standardized_types::base_data::quotebar::generate_5_day_quote_bar_data;
    use crate::standardized_types::datavendor_enum::DataVendor;
    use crate::standardized_types::resolution::Resolution;
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
    async fn test_daily_quote_consolidation() {
        let trading_hours = setup_trading_hours();
        let subscription = DataSubscription {
            symbol: Symbol::new("TEST".to_string(), DataVendor::DataBento, MarketType::CFD),
            base_data_type: BaseDataType::QuoteBars,
            resolution: Resolution::Day,
            candle_type: Some(CandleType::CandleStick),
            market_type: MarketType::CFD,
        };

        // Get the start time from the test data and print it
        let test_data = generate_5_day_quote_bar_data();
        let first_time = parse_datetime(&test_data.first().unwrap().time);
        let last_time = parse_datetime(&test_data.last().unwrap().time);

        println!("\nTest data range:");
        println!("First quote bar time: {}", first_time);
        println!("Last quote bar time: {}", last_time);

        let mut consolidator = DailyQuoteConsolidator::new_with_reference_time(
            subscription,
            2,
            dec!(0.01),
            trading_hours,
            first_time,
        ).unwrap();

        // Print session map after initialization
        println!("\nInitial session map:");
        for (start, session) in consolidator.session_map.iter() {
            println!("Session: {} -> {}", start, session.close);
        }

        let mut daily_bars = Vec::new();

        for (i, quote_bar) in test_data.iter().enumerate() {
            let time = parse_datetime(&quote_bar.time);

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

            let result = consolidator.update(&BaseDataEnum::QuoteBar(quote_bar.clone()));
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

        println!("\nGenerated {} daily quote bars:", daily_bars.len());
        for (i, bar) in daily_bars.iter().enumerate() {
            if let BaseDataEnum::QuoteBar(quote_bar) = bar {
                println!("Bar {}: Time={}, Ask(O={},H={},L={},C={}), Bid(O={},H={},L={},C={})",
                         i, quote_bar.time,
                         quote_bar.ask_open, quote_bar.ask_high, quote_bar.ask_low, quote_bar.ask_close,
                         quote_bar.bid_open, quote_bar.bid_high, quote_bar.bid_low, quote_bar.bid_close
                );
            }
        }

        assert!(!daily_bars.is_empty(), "Should produce at least one bar");

        let market_data: Vec<&QuoteBar> = test_data.iter()
            .filter(|qb| {
                let time = parse_datetime(&qb.time);
                consolidator.get_current_session(time).is_some()
            })
            .collect();

        println!("\nFound {} quote bars during market hours", market_data.len());

        for (i, bar) in daily_bars.iter().enumerate() {
            if let BaseDataEnum::QuoteBar(quote_bar) = bar {
                println!("Verifying bar {}: {}", i, quote_bar.time);

                let bar_time = parse_datetime(&quote_bar.time);

                if let Some(session) = consolidator.get_current_session(bar_time) {
                    let session_data: Vec<&QuoteBar> = market_data.iter()
                        .filter(|qb| {
                            let time = parse_datetime(&qb.time);
                            time >= session.open && time < session.close
                        })
                        .copied()
                        .collect();

                    if !session_data.is_empty() {
                        let expected_ask_high = session_data.iter().map(|qb| qb.ask_high).max().unwrap();
                        let expected_ask_low = session_data.iter().map(|qb| qb.ask_low).min().unwrap();
                        let expected_bid_high = session_data.iter().map(|qb| qb.bid_high).max().unwrap();
                        let expected_bid_low = session_data.iter().map(|qb| qb.bid_low).min().unwrap();
                        let expected_volume: Decimal = session_data.iter().map(|qb| qb.volume).sum();
                        let expected_ask_volume: Decimal = session_data.iter().map(|qb| qb.ask_volume).sum();
                        let expected_bid_volume: Decimal = session_data.iter().map(|qb| qb.bid_volume).sum();

                        assert_eq!(quote_bar.ask_high, expected_ask_high, "Bar {} ask high mismatch", i);
                        assert_eq!(quote_bar.ask_low, expected_ask_low, "Bar {} ask low mismatch", i);
                        assert_eq!(quote_bar.bid_high, expected_bid_high, "Bar {} bid high mismatch", i);
                        assert_eq!(quote_bar.bid_low, expected_bid_low, "Bar {} bid low mismatch", i);
                        assert_eq!(quote_bar.volume, expected_volume, "Bar {} volume mismatch", i);
                        assert_eq!(quote_bar.ask_volume, expected_ask_volume, "Bar {} ask volume mismatch", i);
                        assert_eq!(quote_bar.bid_volume, expected_bid_volume, "Bar {} bid volume mismatch", i);
                    }
                }
            }
        }
    }
}