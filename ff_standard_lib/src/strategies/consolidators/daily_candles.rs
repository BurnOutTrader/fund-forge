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

pub struct DailyConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    decimal_accuracy: u32,
    tick_size: Decimal,
    last_close: Option<Price>,
    last_ask_close: Option<Price>,
    last_bid_close: Option<Price>,
    fill_forward: bool,
    market_type: MarketType,
    last_bar_open: DateTime<Utc>,
    trading_hours: TradingHours,
    days_per_bar: i64,
    current_bar_start_day: Option<DateTime<Utc>>,
}
#[allow(dead_code)]
impl DailyConsolidator {
    pub(crate) async fn new(
        subscription: DataSubscription,
        fill_forward: bool,
        decimal_accuracy: u32,
        tick_size: Decimal,
        trading_hours: TradingHours,
    ) -> Result<Self, FundForgeError> {
        println!("Creating Daily Consolidator For: {}", subscription);

        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                "{} is an Invalid base data type for DailyConsolidator",
                subscription.base_data_type
            )));
        }

        let days_per_bar = match subscription.resolution {
            Resolution::Days(days) => days as i64,
            _ => return Err(FundForgeError::ClientSideErrorDebug(
                "DailyConsolidator requires Resolution::Daily".to_string()
            )),
        };

        let market_type = subscription.symbol.market_type.clone();

        Ok(DailyConsolidator {
            current_data: None,
            market_type,
            subscription,
            decimal_accuracy,
            tick_size,
            last_close: None,
            last_ask_close: None,
            last_bid_close: None,
            fill_forward,
            last_bar_open: DateTime::<Utc>::MIN_UTC,
            trading_hours,
            days_per_bar,
            current_bar_start_day: None,
        })
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

    fn get_next_market_open(&self, from_time: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut check_time = from_time.with_timezone(&self.trading_hours.timezone);

        // Try for up to 14 days to find the next market open (for multi-day bars)
        for _ in 0..14 {
            let current_session = self.get_session_for_day(check_time.weekday());

            if let Some(open_time) = current_session.open {
                let current_time = check_time.time();
                let market_datetime = if current_time >= open_time {
                    // If we've passed today's open, look at next day
                    check_time.date_naive().succ_opt().unwrap()
                        .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                        .unwrap()
                } else {
                    // Use today's open time
                    check_time.date_naive()
                        .and_hms_opt(open_time.hour(), open_time.minute(), open_time.second())
                        .unwrap()
                };

                // Convert to timezone-aware datetime
                if let Some(tz_datetime) = self.trading_hours.timezone.from_local_datetime(&market_datetime).latest() {
                    return Some(tz_datetime.with_timezone(&Utc));
                }
            }

            // Move to next day
            check_time = check_time.date_naive().succ_opt().unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(self.trading_hours.timezone)
                .unwrap();
        }

        None
    }

    fn should_start_new_bar(&self, time: DateTime<Utc>) -> bool {
        if self.current_bar_start_day.is_none() {
            return true;
        }

        let start_day = self.current_bar_start_day.unwrap();
        let days_elapsed = (time - start_day).num_days();

        days_elapsed >= self.days_per_bar
    }

    fn is_session_end(&self, time: DateTime<Utc>) -> bool {
        let market_time = time.with_timezone(&self.trading_hours.timezone);
        let session = self.get_session_for_day(market_time.weekday());

        if let Some(close_time) = session.close {
            // If there's a close time, check if we've reached it
            market_time.time() >= close_time
        } else {
            // For sessions without close time (24h trading),
            // check if the next session exists and has different trading hours
            let next_day = market_time + Duration::days(1);
            let next_session = self.get_session_for_day(next_day.weekday());

            match (session.open, next_session.open) {
                (Some(curr_open), Some(next_open)) => {
                    // If next session has different hours, consider this a boundary
                    curr_open != next_open
                }
                (None, Some(_)) | (Some(_), None) => true, // Trading hours change
                (None, None) => false, // Both closed
            }
        }
    }

    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        // If market is closed, don't update
        if !self.trading_hours.is_market_open(time) {
            return None;
        }

        if let Some(current_bar) = &self.current_data {
            if time < current_bar.time_utc() {
                return None;
            }
        }

        // Evaluate session end condition before taking mutable borrow
        let should_close = if self.current_data.is_some() {
            self.is_session_end(time) || self.should_start_new_bar(time)
        } else {
            false
        };

        if should_close {
            // Now we can safely take the mutable borrow
            if let Some(current_data) = self.current_data.as_mut() {
                let mut return_data = current_data.clone();
                return_data.set_is_closed(true);
                self.current_data = None;
                self.fill_forward(time);
                return Some(return_data);
            }
        } else if self.current_data.is_none() {
            self.fill_forward(time);
        }
        None
    }

    fn fill_forward(&mut self, time: DateTime<Utc>) {
        if self.fill_forward {
            match self.subscription.base_data_type {
                BaseDataType::Candles => {
                    if let Some(last_close) = self.last_close {
                        let time = if let Some(next_open) = self.get_next_market_open(time) {
                            next_open
                        } else {
                            time
                        };

                        if time == self.last_bar_open {
                            return;
                        }

                        self.last_bar_open = time.clone();
                        self.current_data = Some(BaseDataEnum::Candle(Candle {
                            symbol: self.subscription.symbol.clone(),
                            open: last_close,
                            high: last_close,
                            low: last_close,
                            close: last_close,
                            volume: dec!(0.0),
                            ask_volume: dec!(0.0),
                            bid_volume: dec!(0.0),
                            time: time.to_string(),
                            resolution: self.subscription.resolution.clone(),
                            is_closed: false,
                            range: dec!(0.0),
                            candle_type: CandleType::CandleStick,
                        }));
                    }
                }
                _ => {} // We only support fill-forward for candles in this consolidator
            }
        }
    }

    fn update_candles(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        // Only process data when market is open
        if !self.trading_hours.is_market_open(base_data.time_utc()) {
            return ConsolidatedData::with_open(base_data.clone());
        }

        if self.current_data.is_none() {
            let data = self.new_candle(base_data);
            self.current_bar_start_day = Some(base_data.time_utc());
            self.current_data = Some(BaseDataEnum::Candle(data.clone()));
            return ConsolidatedData::with_open(BaseDataEnum::Candle(data));
        }

        let time = base_data.time_utc();

        // Check time ordering before any other operations
        if let Some(current_bar) = &self.current_data {
            if time < current_bar.time_utc() {
                return ConsolidatedData::with_open(base_data.clone());
            }
        }

        // Evaluate session end condition before taking mutable borrow
        let should_close = self.is_session_end(time) || self.should_start_new_bar(time);

        if should_close {
            // Now we can safely handle the closing of the bar
            if let Some(current_bar) = self.current_data.as_mut() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);

                // Store last close before we create new bar
                match &consolidated_bar {
                    BaseDataEnum::Candle(candle) => {
                        self.last_close = Some(candle.close.clone());
                    }
                    _ => {}
                }

                let new_bar = self.new_candle(base_data);
                self.current_bar_start_day = Some(time);
                self.current_data = Some(BaseDataEnum::Candle(new_bar.clone()));

                return ConsolidatedData::with_closed(
                    BaseDataEnum::Candle(new_bar),
                    consolidated_bar
                );
            }
        }

        // Update existing bar
        if let Some(current_bar) = self.current_data.as_mut() {
            match current_bar {
                BaseDataEnum::Candle(candle) => match base_data {
                    BaseDataEnum::Tick(tick) => {
                        candle.high = candle.high.max(tick.price);
                        candle.low = candle.low.min(tick.price);
                        candle.close = tick.price;
                        candle.range = self.market_type.round_price(
                            candle.high - candle.low,
                            self.tick_size,
                            self.decimal_accuracy,
                        );

                        match tick.aggressor {
                            Aggressor::Buy => candle.bid_volume += tick.volume,
                            Aggressor::Sell => candle.ask_volume += tick.volume,
                            _ => {}
                        }

                        candle.volume += tick.volume;
                        ConsolidatedData::with_open(base_data.clone())
                    }
                    BaseDataEnum::Candle(new_candle) => {
                        candle.high = candle.high.max(new_candle.high);
                        candle.low = candle.low.min(new_candle.low);
                        candle.close = new_candle.close;
                        candle.range = self.market_type.round_price(
                            candle.high - candle.low,
                            self.tick_size,
                            self.decimal_accuracy,
                        );
                        candle.volume += new_candle.volume;
                        candle.ask_volume += new_candle.ask_volume;
                        candle.bid_volume += new_candle.bid_volume;
                        ConsolidatedData::with_open(base_data.clone())
                    }
                    _ => panic!(
                        "Invalid base data type for Candle consolidator: {}",
                        base_data.base_data_type()
                    ),
                },
                _ => panic!(
                    "Invalid base data type for Candle consolidator: {}",
                    base_data.base_data_type()
                ),
            }
        } else {
            panic!(
                "Invalid base data type for Candle consolidator: {}",
                base_data.base_data_type()
            );
        }
    }

    fn new_candle(&mut self, new_data: &BaseDataEnum) -> Candle {
        let time = if let Some(next_open) = self.get_next_market_open(new_data.time_utc()) {
            next_open
        } else {
            new_data.time_utc() // Fallback to data time if we can't determine next market open
        };

        self.last_bar_open = time;

        match new_data {
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
                    time.to_string(),
                    self.subscription.resolution.clone(),
                    self.subscription.candle_type.clone().unwrap(),
                )
            }
            BaseDataEnum::Candle(candle) => {
                let mut consolidated_candle = candle.clone();
                consolidated_candle.is_closed = false;
                consolidated_candle.resolution = self.subscription.resolution.clone();
                consolidated_candle.time = time.to_string();
                consolidated_candle
            }
            _ => panic!("Invalid base data type for Candle consolidator"),
        }
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        match base_data.base_data_type() {
            BaseDataType::Ticks | BaseDataType::Candles => self.update_candles(base_data),
            _ => panic!("Only Ticks and Candles are supported for daily consolidation"),
        }
    }
}