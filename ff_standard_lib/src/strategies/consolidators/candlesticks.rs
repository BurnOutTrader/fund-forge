use crate::helpers::converters;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::{MarketType, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;
use crate::helpers::converters::open_time;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::base_data::tick::Aggressor;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::resolution::Resolution;

pub struct CandleStickConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    decimal_accuracy: u32,
    tick_size: Decimal,
    last_close: Option<Price>,
    last_ask_close: Option<Price>,
    last_bid_close: Option<Price>,
    fill_forward: bool,
    market_type: MarketType,
    subscription_resolution_type: SubscriptionResolutionType,
}

impl CandleStickConsolidator {
    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        if let Some(current_bar) = &self.current_data {
            if time < current_bar.time_utc() {
                return None;
            }
        }
        if self.fill_forward && self.current_data == None {
            match self.subscription.base_data_type {
                BaseDataType::QuoteBars => {
                    if let (Some(last_bid_close), Some(last_ask_close)) = (self.last_bid_close, self.last_ask_close) {
                        let time = open_time(&self.subscription, time);
                        let spread = self.market_type.round_price(last_ask_close - last_bid_close, self.tick_size, self.decimal_accuracy);

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
                            ask_volume: dec!(0.0),
                            bid_volume: dec!(0.0),
                            time: open_time(&self.subscription, time).to_string(),
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
                            ask_volume: dec!(0.0),
                            bid_volume: dec!(0.0),
                            time: open_time(&self.subscription, time).to_string(),
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
            const NANO: Duration = Duration::nanoseconds(1);
            if time > current_data.time_closed_utc() - NANO {
                let mut return_data = current_data.clone();
                return_data.set_is_closed(true);
                self.current_data = None;
                return Some(return_data)
            }
        }
        None
    }


    fn update_candles(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        if base_data.subscription().subscription_resolution_type() != self.subscription_resolution_type {
            panic!("Unsupported type") //todo remove this check on final builds
        }
        if self.current_data.is_none() {
            let data = self.new_candle(base_data);
            self.current_data = Some(BaseDataEnum::Candle(data));
            return ConsolidatedData::with_open(self.current_data.clone().unwrap())
        }
        else if let Some(current_bar) = self.current_data.as_mut() {
            let time = base_data.time_closed_utc();
            if time < current_bar.time_utc() {
                return ConsolidatedData::with_open(current_bar.clone());
            }
            const NANO: Duration = Duration::nanoseconds(1);
            if base_data.time_closed_utc() >= current_bar.time_closed_utc() - NANO {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                match &consolidated_bar {
                    BaseDataEnum::Candle(candle) => {
                        self.last_close = Some(candle.close.clone());
                    }
                    _ => {}
                }
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
                        candle.range = self.market_type.round_price(candle.high - candle.low, self.tick_size, self.decimal_accuracy);

                        match tick.aggressor {
                            Aggressor::Buy => candle.bid_volume += tick.volume,
                            Aggressor::Sell => candle.ask_volume += tick.volume,
                            _ => {}
                        }

                        candle.volume += tick.volume;
                        return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                    }
                    BaseDataEnum::Candle(new_candle) => {
                        candle.high = candle.high.max(new_candle.high);
                        candle.low = candle.low.min(new_candle.low);
                        candle.range = self.market_type.round_price(candle.high - candle.low, self.tick_size, self.decimal_accuracy);
                        candle.close = new_candle.close;
                        candle.volume += new_candle.volume;
                        candle.ask_volume += new_candle.ask_volume;
                        candle.bid_volume += new_candle.bid_volume;
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
            BaseDataEnum::Quote(quote) => {
                QuoteBar::new(
                    self.subscription.symbol.clone(),
                    quote.bid,
                    quote.ask,
                    quote.bid_volume + quote.bid_volume,
                    quote.ask_volume,
                    quote.bid_volume,
                    time.to_string(),
                    self.subscription.resolution.clone(),
                    CandleType::CandleStick,
                )
            },
            _ => panic!("Invalid base data type for QuoteBar consolidator"),
        }
    }

    /// We can use if time == some multiple of resolution then we can consolidate, we dont need to know the actual algo time, because we can get time from the base_data if self.last_time >
    fn update_quote_bars(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        if base_data.subscription().subscription_resolution_type() != self.subscription_resolution_type {
            panic!("Unsupported type") //todo remove this check on final builds
        }
        if self.current_data.is_none() {
            let data = self.new_quote_bar(base_data);
            self.current_data = Some(BaseDataEnum::QuoteBar(data));
            return ConsolidatedData::with_open(self.current_data.clone().unwrap())
        } else if let Some(current_bar) = self.current_data.as_mut() {
            let time = base_data.time_utc();
            if time < current_bar.time_utc() {
                return ConsolidatedData::with_open(current_bar.clone());
            }
            const NANO: Duration = Duration::nanoseconds(1);
            if base_data.time_closed_utc() >= current_bar.time_closed_utc() - NANO {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
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
                            quote_bar.volume += quote.ask_volume + quote.bid_volume;
                            quote_bar.bid_volume += quote.bid_volume;
                            quote_bar.ask_volume += quote.ask_volume;
                            quote_bar.range = self.market_type.round_price(quote_bar.ask_high - quote_bar.bid_low, self.tick_size, self.decimal_accuracy);
                            quote_bar.spread = self.market_type.round_price(quote_bar.ask_close - quote_bar.bid_close, self.tick_size, self.decimal_accuracy);
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
                            quote_bar.bid_volume += bar.bid_volume;
                            quote_bar.ask_volume += bar.ask_volume;
                            quote_bar.range = self.market_type.round_price(quote_bar.ask_high - quote_bar.bid_low, self.tick_size, self.decimal_accuracy);
                            quote_bar.spread = self.market_type.round_price(quote_bar.ask_close - quote_bar.bid_close, self.tick_size, self.decimal_accuracy);
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
            BaseDataEnum::Tick(tick) => {
                let (ask_volume, bid_volume) = match tick.aggressor {
                    Aggressor::Buy => (dec!(0.0), tick.volume),
                    Aggressor::Sell => (tick.volume, dec!(0.0)),
                    Aggressor::None => (dec!(0), dec!(0))
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
            },
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

    pub(crate) async fn new(
        subscription: DataSubscription,
        fill_forward: bool,
        subscription_resolution_type: SubscriptionResolutionType
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

        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone()).await?;
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone()).await?;

        let market_type = subscription.symbol.market_type.clone();

        Ok(CandleStickConsolidator {
            current_data: None,
            market_type,
            subscription,
            decimal_accuracy,
            tick_size,
            last_close: None,
            last_ask_close: None,
            last_bid_close: None,
            fill_forward,
            subscription_resolution_type,
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
            BaseDataType::QuoteBars => {
                self.update_quote_bars(base_data)
            }
            BaseDataType::Candles => {
                self.update_candles(base_data)
            }
            BaseDataType::Fundamentals => panic!("Fundamentals are not supported"),
        }
    }
}
