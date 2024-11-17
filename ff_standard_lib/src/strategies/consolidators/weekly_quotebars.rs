use chrono::{DateTime, Datelike, Duration, NaiveTime, Timelike, Utc, Weekday};
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

pub struct WeeklyQuoteConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    decimal_accuracy: u32,
    tick_size: Decimal,
    last_ask_close: Option<Price>,
    last_bid_close: Option<Price>,
    market_type: MarketType,
    trading_hours: TradingHours,
    week_start_day: Weekday,
    week_start_session: Option<(Weekday, NaiveTime)>,
    week_end_session: Option<(Weekday, NaiveTime)>,
}

impl WeeklyQuoteConsolidator {
    pub(crate) async fn new(
        subscription: DataSubscription,
        decimal_accuracy: u32,
        tick_size: Decimal,
        trading_hours: TradingHours,
        week_start_day: Weekday,
    ) -> Result<Self, FundForgeError> {
        match subscription.base_data_type {
            BaseDataType::Quotes | BaseDataType::QuoteBars => {},
            _ => return Err(FundForgeError::ClientSideErrorDebug(format!(
                "{} is an Invalid base data type for WeeklyQuoteConsolidator",
                subscription.base_data_type
            )))
        }

        let market_type = subscription.symbol.market_type.clone();
        let (week_start_session, week_end_session) = Self::calculate_week_schedule(&trading_hours, week_start_day);

        Ok(WeeklyQuoteConsolidator {
            current_data: None,
            subscription,
            decimal_accuracy,
            tick_size,
            last_ask_close: None,
            last_bid_close: None,
            market_type,
            trading_hours,
            week_start_day,
            week_start_session,
            week_end_session,
        })
    }

    fn calculate_week_schedule(trading_hours: &TradingHours, week_start: Weekday)
                               -> (Option<(Weekday, NaiveTime)>, Option<(Weekday, NaiveTime)>) {
        let session_days = [
            (Weekday::Mon, &trading_hours.monday),
            (Weekday::Tue, &trading_hours.tuesday),
            (Weekday::Wed, &trading_hours.wednesday),
            (Weekday::Thu, &trading_hours.thursday),
            (Weekday::Fri, &trading_hours.friday),
            (Weekday::Sat, &trading_hours.saturday),
            (Weekday::Sun, &trading_hours.sunday),
        ];

        // Reorder days to start from configured week start
        let mut ordered_days = Vec::with_capacity(7);
        let start_idx = session_days.iter().position(|(day, _)| *day == week_start).unwrap();
        ordered_days.extend(&session_days[start_idx..]);
        ordered_days.extend(&session_days[..start_idx]);

        let mut first_session = None;
        let mut last_session = None;

        // Find first trading session
        for &(day, session) in ordered_days.iter() {
            if let Some(open_time) = session.open {
                first_session = Some((day, open_time));
                break;
            }
        }

        // Find last trading session
        for &(day, session) in ordered_days.iter().rev() {
            if let Some(close_time) = session.close {
                if session.open.is_some() {
                    last_session = Some((day, close_time));
                    break;
                }
            }
        }

        (first_session, last_session)
    }

    fn get_trading_sessions(&self) -> Vec<(Weekday, &DaySession)> {
        let session_days = [
            (Weekday::Mon, &self.trading_hours.monday),
            (Weekday::Tue, &self.trading_hours.tuesday),
            (Weekday::Wed, &self.trading_hours.wednesday),
            (Weekday::Thu, &self.trading_hours.thursday),
            (Weekday::Fri, &self.trading_hours.friday),
            (Weekday::Sat, &self.trading_hours.saturday),
            (Weekday::Sun, &self.trading_hours.sunday),
        ];

        // Reorder starting from configured week start
        let mut ordered_days = Vec::with_capacity(7);
        let start_idx = session_days.iter().position(|(day, _)| *day == self.week_start_day).unwrap();
        ordered_days.extend(&session_days[start_idx..]);
        ordered_days.extend(&session_days[..start_idx]);

        // Only include days that have trading sessions
        ordered_days.into_iter()
            .filter(|(_, session)| session.open.is_some())
            .collect()
    }

    fn is_week_end(&self, time: DateTime<Utc>) -> bool {
        let market_time = time.with_timezone(&self.trading_hours.timezone);

        if let Some((end_day, end_time)) = self.week_end_session {
            if market_time.weekday() == end_day && market_time.time() >= end_time {
                // Verify this is actually the last trading session of the week
                let sessions = self.get_trading_sessions();
                if let Some((last_day, _)) = sessions.last() {
                    return end_day == *last_day;
                }
            }
        }

        false
    }

    fn get_week_start(&self, time: DateTime<Utc>) -> DateTime<Utc> {
        let market_time = time.with_timezone(&self.trading_hours.timezone);
        let sessions = self.get_trading_sessions();

        // Find the appropriate session based on the current time
        for (idx, &(day, session)) in sessions.iter().enumerate() {
            if let Some(open_time) = session.open {
                let market_day_num = market_time.weekday().num_days_from_sunday();
                let session_day_num = day.num_days_from_sunday();

                if market_day_num < session_day_num ||
                    (market_time.weekday() == day && market_time.time() < open_time) {
                    // If we're before this session, use previous week's first session
                    if let Some(&(first_day, first_session)) = sessions.first() {
                        if let Some(first_open) = first_session.open {
                            let mut start_time = market_time;
                            // Move back to previous week's start
                            while start_time.weekday() != first_day {
                                start_time = start_time - Duration::days(1);
                            }
                            return start_time.date_naive()
                                .and_hms_opt(first_open.hour(), first_open.minute(), first_open.second())
                                .unwrap()
                                .and_local_timezone(self.trading_hours.timezone)
                                .unwrap()
                                .with_timezone(&Utc);
                        }
                    }
                }

                if market_time.weekday() == day {
                    // We're in the current session
                    return market_time.date_naive()
                        .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                        .unwrap()
                        .and_local_timezone(self.trading_hours.timezone)
                        .unwrap()
                        .with_timezone(&Utc);
                }

                if idx == 0 && market_day_num > session_day_num {
                    // We're past all sessions, use next week's first session
                    let mut start_time = market_time;
                    while start_time.weekday() != day {
                        start_time = start_time + Duration::days(1);
                    }
                    return start_time.date_naive()
                        .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                        .unwrap()
                        .and_local_timezone(self.trading_hours.timezone)
                        .unwrap()
                        .with_timezone(&Utc);
                }
            }
        }

        // Fallback to first session of current week
        if let Some(&(first_day, first_session)) = sessions.first() {
            if let Some(open_time) = first_session.open {
                let mut start_time = market_time;
                while start_time.weekday() != first_day {
                    start_time = start_time - Duration::days(1);
                }
                return start_time.date_naive()
                    .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                    .unwrap()
                    .and_local_timezone(self.trading_hours.timezone)
                    .unwrap()
                    .with_timezone(&Utc);
            }
        }

        market_time.with_timezone(&Utc)
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        if !self.trading_hours.is_market_open(base_data.time_utc()) {
            return ConsolidatedData::with_open(base_data.clone());
        }

        if self.current_data.is_none() {
            let time = self.get_week_start(base_data.time_utc());
            let data = self.new_quote_bar(base_data, time);
            self.current_data = Some(BaseDataEnum::QuoteBar(data.clone()));
            return ConsolidatedData::with_open(BaseDataEnum::QuoteBar(data));
        }

        let time = base_data.time_utc();

        if let Some(current_bar) = &self.current_data {
            if time < current_bar.time_utc() {
                return ConsolidatedData::with_open(base_data.clone());
            }
        }

        let should_close = self.is_week_end(time);

        if should_close {
            if let Some(current_bar) = self.current_data.as_mut() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);

                if let BaseDataEnum::QuoteBar(quote_bar) = &consolidated_bar {
                    self.last_ask_close = Some(quote_bar.ask_close.clone());
                    self.last_bid_close = Some(quote_bar.bid_close.clone());
                }

                let week_start = self.get_week_start(time);
                let new_bar = self.new_quote_bar(base_data, week_start);
                self.current_data = Some(BaseDataEnum::QuoteBar(new_bar.clone()));

                return ConsolidatedData::with_closed(
                    BaseDataEnum::QuoteBar(new_bar),
                    consolidated_bar
                );
            }
        }

        if let Some(current_bar) = self.current_data.as_mut() {
            match current_bar {
                BaseDataEnum::QuoteBar(quote_bar) => match base_data {
                    BaseDataEnum::Quote(quote) => {
                        quote_bar.ask_high = quote_bar.ask_high.max(quote.ask);
                        quote_bar.ask_low = quote_bar.ask_low.min(quote.ask);
                        quote_bar.bid_high = quote_bar.bid_high.max(quote.bid);
                        quote_bar.bid_low = quote_bar.bid_low.min(quote.bid);
                        quote_bar.ask_close = quote.ask;
                        quote_bar.bid_close = quote.bid;
                        quote_bar.volume += quote.ask_volume + quote.bid_volume;
                        quote_bar.ask_volume += quote.ask_volume;
                        quote_bar.bid_volume += quote.bid_volume;
                        quote_bar.range = self.market_type.round_price(
                            quote_bar.ask_high - quote_bar.bid_low,
                            self.tick_size,
                            self.decimal_accuracy,
                        );
                        quote_bar.spread = self.market_type.round_price(
                            quote_bar.ask_close - quote_bar.bid_close,
                            self.tick_size,
                            self.decimal_accuracy,
                        );
                        ConsolidatedData::with_open(base_data.clone())
                    }
                    BaseDataEnum::QuoteBar(new_quote_bar) => {
                        quote_bar.ask_high = quote_bar.ask_high.max(new_quote_bar.ask_high);
                        quote_bar.ask_low = quote_bar.ask_low.min(new_quote_bar.ask_low);
                        quote_bar.bid_high = quote_bar.bid_high.max(new_quote_bar.bid_high);
                        quote_bar.bid_low = quote_bar.bid_low.min(new_quote_bar.bid_low);
                        quote_bar.ask_close = new_quote_bar.ask_close;
                        quote_bar.bid_close = new_quote_bar.bid_close;
                        quote_bar.volume += new_quote_bar.volume;
                        quote_bar.ask_volume += new_quote_bar.ask_volume;
                        quote_bar.bid_volume += new_quote_bar.bid_volume;
                        quote_bar.range = self.market_type.round_price(
                            quote_bar.ask_high - quote_bar.bid_low,
                            self.tick_size,
                            self.decimal_accuracy,
                        );
                        quote_bar.spread = self.market_type.round_price(
                            quote_bar.ask_close - quote_bar.bid_close,
                            self.tick_size,
                            self.decimal_accuracy,
                        );
                        ConsolidatedData::with_open(base_data.clone())
                    }
                    _ => panic!("Invalid base data type for quote bar consolidator"),
                },
                _ => panic!("Invalid current bar type for quote bar consolidator"),
            }
        } else {
            panic!("Current bar is None after checking for Some");
        }
    }

    fn new_quote_bar(&mut self, base_data: &BaseDataEnum, time: DateTime<Utc>) -> QuoteBar {
        match base_data {
            BaseDataEnum::Quote(quote) => {
                QuoteBar::new(
                    self.subscription.symbol.clone(),
                    quote.bid,
                    quote.ask,
                    quote.bid_volume + quote.ask_volume,
                    quote.ask_volume,
                    quote.bid_volume,
                    time.to_string(),
                    self.subscription.resolution.clone(),
                    CandleType::CandleStick,
                )
            }
            BaseDataEnum::QuoteBar(quote_bar) => {
                let mut consolidated_bar = quote_bar.clone();
                consolidated_bar.is_closed = false;
                consolidated_bar.resolution = self.subscription.resolution.clone();
                consolidated_bar.time = time.to_string();
                consolidated_bar
            }
            _ => panic!("Invalid base data type for quote bar consolidator"),
        }
    }

    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        if let Some(current_bar) = &self.current_data {
            if time < current_bar.time_utc() {
                return None;
            }
        }

        let should_close = self.is_week_end(time);

        if should_close {
            if let Some(current_data) = self.current_data.as_mut() {
                let mut return_data = current_data.clone();
                return_data.set_is_closed(true);

                if let BaseDataEnum::QuoteBar(quote_bar) = &return_data {
                    self.last_ask_close = Some(quote_bar.ask_close.clone());
                    self.last_bid_close = Some(quote_bar.bid_close.clone());
                }

                self.current_data = None;
                return Some(return_data);
            }
        }

        None
    }
}