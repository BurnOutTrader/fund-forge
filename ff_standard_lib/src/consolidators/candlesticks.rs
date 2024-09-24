use crate::helpers::converters;
use crate::helpers::decimal_calculators::{round_to_tick_size};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::Resolution;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use chrono::{DateTime, Utc};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use crate::consolidators::consolidator_enum::ConsolidatedData;
use crate::helpers::converters::open_time;
use crate::standardized_types::{Price, Volume};
use crate::standardized_types::data_server_messaging::FundForgeError;

pub struct CandleStickConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    pub(crate) history: RollingWindow<BaseDataEnum>,
    tick_size: Price,
    last_close: Option<Price>,
    last_ask_close: Option<Price>,
    last_bid_close: Option<Price>,
    fill_forward: bool
}

impl CandleStickConsolidator {
    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        //todo add fill forward option for this
        if self.fill_forward && self.current_data == None {
            match self.subscription.base_data_type {
                BaseDataType::QuoteBars => {
                    if let (Some(last_bid_close), Some(last_ask_close)) = (self.last_bid_close, self.last_ask_close) {
                        let time = open_time(&self.subscription, time);
                        let spread = round_to_tick_size(last_ask_close - last_bid_close, self.tick_size);

                        self.current_data = Some(BaseDataEnum::QuoteBar(QuoteBar {
                            symbol: self.subscription.symbol.clone(),
                            ask_open: last_ask_close,
                            ask_high: last_ask_close,
                            ask_low: last_ask_close,
                            ask_close: last_ask_close,
                            bid_open: last_bid_close,
                            bid_high: last_bid_close,
                            bid_low: last_bid_close,
                            bid_close: last_bid_close,
                            volume: dec!(0.0),
                            time: time.to_string(),
                            resolution: self.subscription.resolution.clone(),
                            is_closed: false,
                            range: dec!(0.0),
                            candle_type: CandleType::CandleStick,
                            spread,
                        }));
                    }
                }
                BaseDataType::Candles => {
                    if let Some(last_close) = self.last_close {
                        let time = open_time(&self.subscription, time);
                        self.current_data = Some(BaseDataEnum::Candle(Candle {
                            symbol: self.subscription.symbol.clone(),
                            open: last_close,
                            high: last_close,
                            low: last_close,
                            close: last_close,
                            volume: dec!(0.0),
                            time: time.to_string(),
                            resolution: self.subscription.resolution.clone(),
                            is_closed: false,
                            range: dec!(0.0),
                            candle_type: CandleType::CandleStick,
                        }));
                    }
                }
                _ => return None
            }
        }
        if let Some(current_data) = self.current_data.as_mut() {
            if time >= current_data.time_created_utc() {
                let mut return_data = current_data.clone();
                return_data.set_is_closed(true);
                self.current_data = None;
                return Some(return_data)
            }
        }
        None
    }

    fn update_candles(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        if self.current_data.is_none() {
            let data = self.new_candle(base_data);
            self.current_data = Some(BaseDataEnum::Candle(data));
            return ConsolidatedData::with_open(self.current_data.clone().unwrap())
        }
        else if let Some(current_bar) = self.current_data.as_mut() {
            if base_data.time_created_utc() >= current_bar.time_created_utc() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                match &consolidated_bar {
                    BaseDataEnum::Candle(candle) => {
                        self.last_close = Some(candle.close.clone());
                    }
                    _ => {}
                }
                self.history.add(consolidated_bar.clone());
                let new_bar = self.new_candle(base_data);
                self.current_data = Some(BaseDataEnum::Candle(new_bar.clone()));
                return ConsolidatedData::with_closed(BaseDataEnum::Candle(new_bar), consolidated_bar);
            }
            match current_bar {
                BaseDataEnum::Candle(candle) =>
                match base_data {
                    BaseDataEnum::Tick(tick) => {
                        candle.high = candle.high.max(tick.price);
                        candle.low = candle.low.min(tick.price);
                        candle.close = tick.price;
                        candle.range = round_to_tick_size(candle.high - candle.low, self.tick_size);
                        candle.volume += tick.volume;
                        return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                    }
                    BaseDataEnum::Candle(new_candle) => {
                        candle.high = candle.high.max(new_candle.high);
                        candle.low = candle.low.min(new_candle.low);
                        candle.range = round_to_tick_size(candle.high - candle.low, self.tick_size);
                        candle.close = new_candle.close;
                        candle.volume += new_candle.volume;
                        return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                    }
                    BaseDataEnum::TradePrice(price) => {
                        candle.high = candle.high.max(price.price);
                        candle.low = candle.low.min(price.price);
                        candle.range = round_to_tick_size(candle.high - candle.low, self.tick_size);
                        candle.close = price.price;
                        return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
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
        }
        panic!(
            "Invalid base data type for Candle consolidator: {}",
            base_data.base_data_type()
        )
    }

    fn new_quote_bar(&self, new_data: &BaseDataEnum) -> QuoteBar {
        let time = converters::open_time(&self.subscription, new_data.time_utc());
        match new_data {
            BaseDataEnum::QuoteBar(bar) => {
                let mut new_bar = bar.clone();
                new_bar.is_closed = false;
                new_bar.time = time.to_string();
                new_bar.resolution = self.subscription.resolution.clone();
                new_bar
            }
            BaseDataEnum::Quote(quote) => QuoteBar::new(
                self.subscription.symbol.clone(),
                quote.bid,
                quote.ask,
                dec!(0.0),
                time.to_string(),
                self.subscription.resolution.clone(),
                CandleType::CandleStick,
            ),
            _ => panic!("Invalid base data type for QuoteBar consolidator"),
        }
    }

    /// We can use if time == some multiple of resolution then we can consolidate, we dont need to know the actual algo time, because we can get time from the base_data if self.last_time >
    fn update_quote_bars(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        if self.current_data.is_none() {
            let data = self.new_quote_bar(base_data);
            self.current_data = Some(BaseDataEnum::QuoteBar(data));
            return ConsolidatedData::with_open(self.current_data.clone().unwrap())
        } else if let Some(current_bar) = self.current_data.as_mut() {
            if base_data.time_created_utc() >= current_bar.time_created_utc() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                self.history.add(consolidated_bar.clone());
                let new_bar = self.new_quote_bar(base_data);
                match &consolidated_bar {
                    BaseDataEnum::QuoteBar(quote_bar) => {
                        self.last_ask_close = Some(quote_bar.ask_close.clone());
                        self.last_bid_close = Some(quote_bar.bid_close.clone());
                    }
                    _ => {}
                }
                self.current_data = Some(BaseDataEnum::QuoteBar(new_bar.clone()));
                return ConsolidatedData::with_closed(BaseDataEnum::QuoteBar(new_bar), consolidated_bar);
            }
            match current_bar {
                BaseDataEnum::QuoteBar(quote_bar) =>
                    match base_data {
                        BaseDataEnum::Quote(quote) => {
                            quote_bar.ask_high = quote_bar.ask_high.max(quote.ask);
                            quote_bar.ask_low = quote_bar.ask_low.min(quote.ask);
                            quote_bar.bid_high = quote_bar.bid_high.max(quote.bid);
                            quote_bar.bid_low = quote_bar.bid_low.min(quote.bid);
                            quote_bar.ask_close = quote.ask;
                            quote_bar.bid_close = quote.bid;
                            quote_bar.range = round_to_tick_size(
                                quote_bar.ask_high - quote_bar.bid_low,
                                self.tick_size,
                            );
                            quote_bar.spread = round_to_tick_size(
                                quote_bar.ask_close - quote_bar.bid_close,
                                self.tick_size,
                            );
                            return ConsolidatedData::with_open(BaseDataEnum::QuoteBar(quote_bar.clone()))
                        }
                        BaseDataEnum::QuoteBar(bar) => {
                            quote_bar.ask_high = quote_bar.ask_high.max(bar.ask_high);
                            quote_bar.ask_low = quote_bar.ask_low.min(bar.ask_low);
                            quote_bar.bid_high = quote_bar.bid_high.max(bar.bid_high);
                            quote_bar.bid_low = bar.bid_low.min(bar.bid_low);
                            quote_bar.ask_close = bar.ask_close;
                            quote_bar.bid_close = bar.bid_close;
                            quote_bar.volume += bar.volume;
                            quote_bar.range = round_to_tick_size(
                                quote_bar.ask_high - quote_bar.bid_low,
                                self.tick_size,
                            );
                            quote_bar.spread = round_to_tick_size(
                                quote_bar.ask_close - quote_bar.bid_close,
                                self.tick_size,
                            );
                            return ConsolidatedData::with_open(BaseDataEnum::QuoteBar(quote_bar.clone()))
                        }
                        _ => panic!(
                            "Invalid base data type for QuoteBar consolidator: {}",
                            base_data.base_data_type()
                        ),
                },
                _ => panic!(
                    "Invalid base data type for QuoteBar consolidator: {}",
                    base_data.base_data_type()
                ),
            }
        }
        panic!(
            "Invalid base data type for QuoteBar consolidator: {}",
            base_data.base_data_type()
        )
    }

    fn new_candle(&self, new_data: &BaseDataEnum) -> Candle {
        let time = converters::open_time(&self.subscription, new_data.time_utc());
        match new_data {
            BaseDataEnum::Tick(tick) => Candle::new(
                self.subscription.symbol.clone(),
                tick.price,
                tick.volume,
                time.to_string(),
                self.subscription.resolution.clone(),
                self.subscription.candle_type.clone().unwrap(),
            ),
            BaseDataEnum::Candle(candle) => {
                let mut consolidated_candle = candle.clone();
                consolidated_candle.is_closed = false;
                consolidated_candle.resolution = self.subscription.resolution.clone();
                consolidated_candle.time = time.to_string();
                consolidated_candle
            }
            BaseDataEnum::TradePrice(price) => Candle::new(
                self.subscription.symbol.clone(),
                price.price,
                Volume::from_f64(0.0).unwrap(),
                time.to_string(),
                self.subscription.resolution.clone(),
                self.subscription.candle_type.clone().unwrap(),
            ),
            _ => panic!("Invalid base data type for Candle consolidator"),
        }
    }

    pub(crate) async fn new(
        subscription: DataSubscription,
        history_to_retain: u64,
        fill_forward: bool
    ) -> Result<Self, FundForgeError> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                    "{} is an Invalid base data type for TimeConsolidator",
                    subscription.base_data_type
                )),
            );
        }

        if let Resolution::Ticks(_) = subscription.resolution {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                    "{:?} is an Invalid resolution for TimeConsolidator",
                    subscription.resolution
                )),
            );
        }

        let tick_size = match subscription.symbol.tick_size().await {
            Ok(size) => size,
            Err(e) => return Err(e)
        };

        Ok(CandleStickConsolidator {
            current_data: None,
            subscription,
            history: RollingWindow::new(history_to_retain),
            tick_size,
            last_close: None,
            last_ask_close: None,
            last_bid_close: None,
            fill_forward
        })
    }

    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        match base_data.base_data_type() {
            BaseDataType::Ticks => {
                self.update_candles(base_data)
            }
            BaseDataType::Quotes => {
                self.update_quote_bars(base_data)
            }
            BaseDataType::TradePrices => {
               self.update_candles(base_data)
            }
            BaseDataType::QuoteBars => {
                self.update_quote_bars(base_data)
            }
            BaseDataType::Candles => {
                self.update_candles(base_data)
            }
            BaseDataType::Fundamentals => panic!("Fundamentals are not supported"),
        }
    }

    pub(crate) fn history(&self) -> RollingWindow<BaseDataEnum> {
        self.history.clone()
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
}
