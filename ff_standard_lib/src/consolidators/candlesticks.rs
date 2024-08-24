use chrono::{DateTime, Duration, Timelike, Utc};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::subscriptions::DataSubscription;
use crate::consolidators::count::ConsolidatorError;
use crate::standardized_types::base_data::history::range_data;

pub fn open_time(subscription: &DataSubscription, time: DateTime<Utc>) -> DateTime<Utc> {
    match subscription.resolution {
        Resolution::Seconds(interval) => {
            let timestamp = time.timestamp();
            let rounded_timestamp = timestamp - (timestamp % interval as i64);
            DateTime::from_timestamp(rounded_timestamp, 0).unwrap()
        }
        Resolution::Minutes(interval) => {
            let minute = time.minute() as i64;
            let rounded_minute = (minute / interval as i64) * interval as i64;
            let rounded_time = time
                .with_minute(rounded_minute as u32)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap();
            rounded_time
        }
        Resolution::Hours(interval) => {
            let hour = time.hour() as i64;
            let rounded_hour = (hour / interval as i64) * interval as i64;
            let rounded_time = time
                .with_hour(rounded_hour as u32)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap();
            rounded_time
        }
        _ => time, // Handle other resolutions if necessary
    }
}

pub struct CandleStickConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    pub(crate) history: RollingWindow<BaseDataEnum>,
}

impl CandleStickConsolidator {
    pub fn update_time(&mut self, time: DateTime<Utc>) -> Vec<BaseDataEnum> {
        if let Some(current_data) = self.current_data.as_mut() {
            if time >= current_data.time_created_utc() {
                let return_data = current_data.clone();
                self.current_data = None;
                return vec![return_data];
            }
        }
        vec![]
    }
    
    fn update_candles(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        if self.current_data.is_none() {
            let data = self.new_candle(base_data);
            self.current_data = Some(BaseDataEnum::Candle(data));
            let candles = vec![self.current_data.clone().unwrap()];
            return candles
        } else if let Some(current_bar) = self.current_data.as_mut() {
            if base_data.time_created_utc() >= current_bar.time_created_utc() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                self.history.add(consolidated_bar.clone());

                let new_bar = self.new_candle(base_data);
                self.current_data = Some(BaseDataEnum::Candle(new_bar.clone()));
                return vec![consolidated_bar, BaseDataEnum::Candle(new_bar)]
            }
            match current_bar {
                BaseDataEnum::Candle(candle) => {
                    match base_data {
                        BaseDataEnum::Tick(tick) => {
                            candle.high = candle.high.max(tick.price);
                            candle.low = candle.low.min(tick.price);
                            candle.close = tick.price;
                            candle.range = candle.high - candle.low;
                            candle.volume += tick.volume;
                            return vec![BaseDataEnum::Candle(candle.clone())]
                        },
                        BaseDataEnum::Candle(new_candle) => {
                            candle.high = candle.high.max(new_candle.high);
                            candle.low = candle.low.min(new_candle.low);
                            candle.range = candle.high - candle.low;
                            candle.close = new_candle.close;
                            candle.volume += new_candle.volume;
                            return vec![BaseDataEnum::Candle(candle.clone())]
                        },
                        BaseDataEnum::Price(price) => {
                            candle.high = candle.high.max(price.price);
                            candle.low = candle.low.min(price.price);
                            candle.range = candle.high - candle.low;
                            candle.close = price.price;
                            return vec![BaseDataEnum::Candle(candle.clone())]
                        },
                        _ => panic!("Invalid base data type for Candle consolidator: {}", base_data.base_data_type())
                    }
                },
                _ =>  panic!("Invalid base data type for Candle consolidator: {}", base_data.base_data_type())
            }
        }
        panic!("Invalid base data type for Candle consolidator: {}", base_data.base_data_type())
    }

    fn new_quote_bar(&self, new_data: &BaseDataEnum) -> QuoteBar {
        let time = open_time(&self.subscription, new_data.time_utc());
        match new_data {
            BaseDataEnum::QuoteBar(bar) => {
                let mut new_bar = bar.clone();
                new_bar.is_closed = false;
                new_bar.time = time.to_string();
                new_bar.resolution = self.subscription.resolution.clone();
                new_bar
            },
            BaseDataEnum::Quote(quote) => QuoteBar::new(self.subscription.symbol.clone(), quote.bid, quote.ask, 0.0, time.to_string(), self.subscription.resolution.clone()),
            _ => panic!("Invalid base data type for QuoteBar consolidator"),
        }
    }

    /// We can use if time == some multiple of resolution then we can consolidate, we dont need to know the actual algo time, because we can get time from the base_data if self.last_time >
    fn update_quote_bars(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        if self.current_data.is_none() {
            let data = self.new_quote_bar(base_data);
            self.current_data = Some(BaseDataEnum::QuoteBar(data));
            return vec![self.current_data.clone().unwrap()]
        } else if let Some(current_bar) = self.current_data.as_mut() {
            if base_data.time_created_utc() >= current_bar.time_created_utc() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                self.history.add(consolidated_bar.clone());
                let new_bar = self.new_quote_bar(base_data);
                self.current_data = Some(BaseDataEnum::QuoteBar(new_bar.clone()));
                return vec![consolidated_bar, BaseDataEnum::QuoteBar(new_bar)]
            }
            match current_bar {
                BaseDataEnum::QuoteBar(quote_bar) => {
                    match base_data {
                        BaseDataEnum::Quote(quote) => {
                            quote_bar.ask_high = quote_bar.ask_high.max(quote.ask);
                            quote_bar.ask_low = quote_bar.ask_low.min(quote.ask);
                            quote_bar.bid_high = quote_bar.bid_high.max(quote.bid);
                            quote_bar.bid_low = quote_bar.bid_low.min(quote.bid);
                            quote_bar.range = quote_bar.ask_high - quote_bar.bid_low;
                            quote_bar.ask_close = quote.ask;
                            quote_bar.bid_close = quote.bid;
                            return vec![BaseDataEnum::QuoteBar(quote_bar.clone())]
                        },
                        BaseDataEnum::QuoteBar(bar) => {
                            quote_bar.ask_high = quote_bar.ask_high.max(bar.ask_high);
                            quote_bar.ask_low = quote_bar.ask_low.min(bar.ask_low);
                            quote_bar.bid_high = quote_bar.bid_high.max(bar.bid_high);
                            quote_bar.bid_low = bar.bid_low.min(bar.bid_low);
                            quote_bar.range = quote_bar.ask_high - quote_bar.bid_low;
                            quote_bar.ask_close = bar.ask_close;
                            quote_bar.bid_close = bar.bid_close;
                            quote_bar.volume += bar.volume;
                            return vec![BaseDataEnum::QuoteBar(quote_bar.clone())]
                        },
                        _ =>  panic!("Invalid base data type for QuoteBar consolidator: {}", base_data.base_data_type())

                    }
                }
                _ =>  panic!("Invalid base data type for QuoteBar consolidator: {}", base_data.base_data_type())
            }
        }
        panic!("Invalid base data type for QuoteBar consolidator: {}", base_data.base_data_type())
    }

    fn new_candle(&self, new_data: &BaseDataEnum) -> Candle {
        let time = open_time(&self.subscription, new_data.time_utc());
        match new_data {
            BaseDataEnum::Tick(tick) => Candle::new(self.subscription.symbol.clone(), tick.price, tick.volume, time.to_string(), self.subscription.resolution.clone(), self.subscription.candle_type.clone().unwrap()),
            BaseDataEnum::Candle(candle) => {
                let mut consolidated_candle = candle.clone();
                consolidated_candle.is_closed = false;
                consolidated_candle.resolution = self.subscription.resolution.clone();
                consolidated_candle.time = time.to_string();
                consolidated_candle
            },
            BaseDataEnum::Price(price) => Candle::new(self.subscription.symbol.clone(), price.price, 0.0, time.to_string(), self.subscription.resolution.clone(), self.subscription.candle_type.clone().unwrap()),
            _ => panic!("Invalid base data type for Candle consolidator")
        }
    }
    
    pub(crate) fn new(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for TimeConsolidator", subscription.base_data_type) });
        }

        if let Resolution::Ticks(_) = subscription.resolution {
            return Err(ConsolidatorError { message: format!("{:?} is an Invalid resolution for TimeConsolidator", subscription.resolution) });
        }

        Ok(CandleStickConsolidator {
            current_data: None,
            subscription,
            history: RollingWindow::new(history_to_retain),
        })
    }

    /// Creates a new TimeConsolidator
    /// 'subscription: DataSubscription' The TimeConsolidator will consolidate data based on the resolution of the subscription.
    /// 'history_to_retain: usize' will retain the last `history_to_retain` bars.
    /// 'warm_up_to_time: DateTime<Utc>' will warm up the history to the specified time.
    /// 'strategy_mode: StrategyMode' will use the specified strategy mode to warm up the history, if Live mode then we will need to get the most recent data from the vendor directly as we may not yet have the data in the serialized history.
    pub(crate) async fn new_and_warmup(subscription: DataSubscription, history_to_retain: usize, warm_up_to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for TimeConsolidator", subscription.base_data_type) });
        }

        if let Resolution::Ticks(_) = subscription.resolution {
            return Err(ConsolidatorError { message: format!("{:?} is an Invalid resolution for TimeConsolidator", subscription.resolution) });
        }
        
        let mut consolidator = CandleStickConsolidator {
            current_data: None,
            subscription,
            history: RollingWindow::new(history_to_retain),
        };
        consolidator.warmup(warm_up_to_time, strategy_mode).await;
        Ok(consolidator)
    }

    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        match base_data.base_data_type() {
            BaseDataType::Ticks => {
                if self.subscription.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            },
            BaseDataType::Quotes => {
                if self.subscription.base_data_type == BaseDataType::QuoteBars {
                    return self.update_quote_bars(base_data);
                }
            },
            BaseDataType::Prices => {
                if self.subscription.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            }
            BaseDataType::QuoteBars => {
                if self.subscription.base_data_type == BaseDataType::QuoteBars {
                    return self.update_quote_bars(base_data);
                }
            }
            BaseDataType::Candles => {
                if self.subscription.base_data_type == BaseDataType::Candles {
                    return self.update_candles(base_data);
                }
            }
            BaseDataType::Fundamentals => panic!("Fundamentals are not supported"),
        }
        vec![]
    }

    pub(crate) fn clear_current_data(&mut self) {
        self.current_data = None;
        self.history.clear();
    }

    fn history(&self) -> &RollingWindow<BaseDataEnum> {
        &self.history
    }


    pub(crate) fn index(&self, index: usize) -> Option<BaseDataEnum> {
        match self.history.get(index) {
            Some(data) => Some(data.clone()),
            None => None,
        }
    }

    pub(crate) fn current(&self) -> Option<BaseDataEnum> {
        match &self.current_data {
            Some(data) => Some(data.clone()),
            None => None,
        }
    }

    async fn warmup(&mut self, to_time: DateTime<Utc>, strategy_mode: StrategyMode) {
        //todo if live we will tell the self.subscription.symbol.data_vendor to .update_historical_symbol()... we will wait then continue
        let vendor_resolutions = self.subscription.symbol.data_vendor.resolutions(self.subscription.market_type.clone()).await.unwrap();
        let mut minimum_resolution: Option<Resolution> = None;
        for resolution in vendor_resolutions {
            if minimum_resolution.is_none() {
                minimum_resolution = Some(resolution);
            } else {
                if resolution > minimum_resolution.unwrap() && resolution < self.subscription.resolution {
                    minimum_resolution = Some(resolution);
                }
            }
        }

        let minimum_resolution = match minimum_resolution.is_none() {
            true => panic!("{} does not have any resolutions available", self.subscription.symbol.data_vendor),
            false => minimum_resolution.unwrap()
        };

        let data_type = match minimum_resolution {
            Resolution::Ticks(_) => BaseDataType::Ticks,
            _ => self.subscription.base_data_type.clone()
        };

        let from_time = to_time - (self.subscription.resolution.as_duration() * self.history().number as i32) - Duration::days(4); //we go back a bit further in case of holidays or weekends

        let base_subscription = DataSubscription::new(self.subscription.symbol.name.clone(), self.subscription.symbol.data_vendor.clone(), minimum_resolution, data_type, self.subscription.market_type.clone());
        let base_data = range_data(from_time, to_time, base_subscription.clone()).await;

        for (_, slice) in &base_data {
            for base_data in slice {
                self.update(base_data);
            }
        }
        if strategy_mode != StrategyMode::Backtest {
            //todo() we will get any bars which are not in out serialized history here
        }
    }
}