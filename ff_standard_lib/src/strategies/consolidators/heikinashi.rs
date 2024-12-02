use crate::helpers::converters::open_time;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::helpers::converters;
use crate::strategies::consolidators::consolidator_enum::ConsolidatedData;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::base_data::tick::Aggressor;
use crate::standardized_types::enums::{MarketType};
use crate::standardized_types::new_types::{Price, Volume};

pub struct HeikinAshiConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    previous_ha_close: Price,
    previous_ha_open: Price,
    decimal_accuracy: u32, //todo, we might need to use tick size to round cme_futures and decimal accuracy to round other products
    tick_size: Decimal,
    fill_forward: bool,
    market_type: MarketType,
    last_bar_open: DateTime<Utc>,
}

impl HeikinAshiConsolidator {
    fn candle_from_base_data(
        &self,
        ha_open: Price,
        ha_high: Price,
        ha_low: Price,
        ha_close: Price,
        volume: Volume,
        ask_volume: Volume,
        bid_volume: Volume,
        time: String,
        is_closed: bool,
        range: Price,
    ) -> Candle {
        Candle {
            symbol: self.subscription.symbol.clone(),
            open: ha_open,
            high: ha_high,
            low: ha_low,
            close: ha_close,
            volume,
            ask_volume,
            bid_volume,
            time,
            resolution: self.subscription.resolution.clone(),
            is_closed,
            range,
            candle_type: CandleType::HeikinAshi,
        }
    }

    fn new_heikin_ashi_candle(&mut self, new_data: &BaseDataEnum) -> Candle {
        let mut time = converters::open_time(&self.subscription, new_data.time_utc());
        if time == self.last_bar_open {
            time += self.subscription.resolution.as_duration();
        }
        self.last_bar_open = time.clone();

        match new_data {
            BaseDataEnum::Candle(candle) => {
                if self.previous_ha_close == dec!(0.0) && self.previous_ha_open == dec!(0.0) {
                    self.previous_ha_close = candle.close;
                    self.previous_ha_open = candle.open;
                }
                let ha_close =  self.market_type.round_price((candle.open + candle.high + candle.low + candle.close) / dec!(4.0), self.tick_size, self.decimal_accuracy);
                let ha_open = self.market_type.round_price((self.previous_ha_open + self.previous_ha_close) / dec!(2.0), self.tick_size, self.decimal_accuracy);
                let ha_high = candle.high.max(ha_open).max(ha_close);
                let ha_low = candle.low.min(ha_open).min(ha_close);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    candle.volume,
                    candle.ask_volume,
                    candle.bid_volume,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            BaseDataEnum::QuoteBar(bar) => {
                if self.previous_ha_close == dec!(0.0) && self.previous_ha_open == dec!(0.0) {
                    self.previous_ha_close = bar.bid_close;
                    self.previous_ha_open = bar.bid_close;
                }
                let ha_close = self.market_type.round_price((bar.bid_open + bar.bid_high + bar.bid_low + bar.bid_close) / dec!(4.0), self.tick_size, self.decimal_accuracy);
                let ha_open = self.market_type.round_price((self.previous_ha_open + self.previous_ha_close) / dec!(2.0), self.tick_size, self.decimal_accuracy);
                let ha_high = ha_close.max(ha_open);
                let ha_low = ha_close.min(ha_open);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    bar.volume,
                    bar.ask_volume,
                    bar.bid_volume,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            BaseDataEnum::Tick(tick) => {
                if self.previous_ha_close == dec!(0.0) && self.previous_ha_open == dec!(0.0) {
                    self.previous_ha_close = tick.price;
                    self.previous_ha_open = tick.price;
                }
                let ha_close = tick.price;
                let ha_open = self.market_type.round_price((self.previous_ha_open + self.previous_ha_close) / dec!(2.0), self.tick_size, self.decimal_accuracy);
                let ha_high = ha_close.max(ha_open);
                let ha_low = ha_close.min(ha_open);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;

                let (ask_volume, bid_volume) = match tick.aggressor {
                    Aggressor::Buy => (dec!(0.0), tick.volume),
                    Aggressor::Sell => (tick.volume, dec!(0.0)),
                    Aggressor::None => (dec!(0), dec!(0))
                };

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    tick.volume,
                    ask_volume,
                    bid_volume,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            BaseDataEnum::Quote(quote) => {
                if self.previous_ha_close == dec!(0.0) && self.previous_ha_open == dec!(0.0) {
                    self.previous_ha_close = quote.bid;
                    self.previous_ha_open = quote.bid;
                }
                let ha_close = quote.bid;
                let ha_open = self.market_type.round_price((self.previous_ha_open + self.previous_ha_close) / dec!(2.0), self.tick_size, self.decimal_accuracy);
                let ha_high = ha_close.max(ha_open);
                let ha_low = ha_close.min(ha_open);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    quote.ask_volume + quote.bid_volume,
                    quote.ask_volume,
                    quote.bid_volume,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            _ => panic!("Invalid base data type for Heikin Ashi calculation"),
        }
    }
}

impl HeikinAshiConsolidator {
    pub(crate) async fn new(
        subscription: DataSubscription,
        fill_forward: bool,
        decimal_accuracy: u32,
        tick_size: Decimal,
    ) -> Result<HeikinAshiConsolidator, FundForgeError> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(FundForgeError::ClientSideErrorDebug(format!(
                    "{} is an Invalid base data type for HeikinAshiConsolidator",
                    subscription.base_data_type
                ),
            ));
        }

        println!("Creating Consolidator For: {}", subscription);
        if let Some(candle_type) = &subscription.candle_type {
            if candle_type != &CandleType::HeikinAshi {
                return Err(FundForgeError::ClientSideErrorDebug(format!(
                        "{:?} is an Invalid candle type for HeikinAshiConsolidator",
                        candle_type
                    ),
                ));
            }
        }

        let market_type = subscription.symbol.market_type.clone();

        Ok(HeikinAshiConsolidator {
            market_type,
            current_data: None,
            subscription,
            previous_ha_close: dec!(0.0),
            previous_ha_open: dec!(0.0),
            decimal_accuracy,
            tick_size,
            fill_forward,
            last_bar_open: DateTime::<Utc>::MIN_UTC,
        })
    }

    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        if let Some(current_bar) = &self.current_data {
            if time < current_bar.time_utc() {
                return None;
            }
        }

        if let Some(current_data) = self.current_data.as_mut() {
            if time > current_data.time_closed_utc()  {
                let mut return_data = current_data.clone();
                return_data.set_is_closed(true);
                self.current_data = None;
                self.fill_forward(time);
                return Some(return_data);
            }
        } else if self.current_data == None {
            self.fill_forward(time);
        }
        None
    }

    fn fill_forward(&mut self, time: DateTime<Utc>) {
        if self.fill_forward {
            let ha_open =  self.market_type.round_price((self.previous_ha_open + self.previous_ha_close) / dec!(2.0), self.tick_size, self.decimal_accuracy);
            let mut time = converters::open_time(&self.subscription, time);
            if time == self.last_bar_open {
                time += self.subscription.resolution.as_duration();
            }
            self.last_bar_open = time.clone();
            self.current_data = Some(BaseDataEnum::Candle(Candle {
                symbol: self.subscription.symbol.clone(),
                open: ha_open,
                high: ha_open,
                low: ha_open,
                close: ha_open,
                volume: dec!(0.0),
                ask_volume: dec!(0.0),
                bid_volume: dec!(0.0),
                time: open_time(&self.subscription, time).to_string(),
                resolution: self.subscription.resolution.clone(),
                is_closed: false,
                range: dec!(0.0),
                candle_type: CandleType::HeikinAshi,
            }));
        }
    }

    //problem where this is returning a closed candle constantly
    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        if self.current_data.is_none() {
            let data = self.new_heikin_ashi_candle(base_data);
            self.current_data = Some(BaseDataEnum::Candle(data));
            return ConsolidatedData::with_open(self.current_data.clone().unwrap())
        } else if let Some(current_bar) = self.current_data.as_mut() {
            let time = base_data.time_closed_utc();
            if time < current_bar.time_utc() {
                // We've already processed data for this time or earlier, so we skip it
                return ConsolidatedData::with_open(current_bar.clone());
            }
            const NANO: Duration = Duration::nanoseconds(1);
            if base_data.time_closed_utc() > current_bar.time_closed_utc() - NANO {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                let new_bar = self.new_heikin_ashi_candle(base_data);
                self.current_data = Some(BaseDataEnum::Candle(new_bar.clone()));
                return ConsolidatedData::with_closed(BaseDataEnum::Candle(new_bar), consolidated_bar);
            }
            match current_bar {
                BaseDataEnum::Candle(candle) =>
                    match base_data {
                        BaseDataEnum::Tick(tick) => {
                            candle.high = tick.price.max(candle.high);
                            candle.low = tick.price.min(candle.low);
                            candle.range = self.market_type.round_price(candle.high - candle.low, self.tick_size, self.decimal_accuracy);
                            candle.volume += tick.volume;
                            match tick.aggressor {
                                Aggressor::Buy => candle.bid_volume += tick.volume,
                                Aggressor::Sell => candle.ask_volume += tick.volume,
                                Aggressor::None => {}
                            };
                            candle.close = self.market_type.round_price((candle.open + candle.high + candle.low + candle.close) / dec!(4.0), self.tick_size, self.decimal_accuracy);
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        BaseDataEnum::Candle(new_candle) => {
                            candle.high = new_candle.high.max(candle.high);
                            candle.low = new_candle.low.min(candle.low);
                            candle.range = self.market_type.round_price(candle.high - candle.low, self.tick_size, self.decimal_accuracy);
                            candle.volume += new_candle.volume;
                            candle.ask_volume += new_candle.ask_volume;
                            candle.bid_volume += new_candle.bid_volume;
                            candle.close = self.market_type.round_price((candle.open + candle.high + candle.low + candle.close) / dec!(4.0), self.tick_size, self.decimal_accuracy);
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        BaseDataEnum::QuoteBar(bar) => {
                            candle.high = bar.bid_high.max(candle.high);
                            candle.low = bar.bid_low.min(candle.low);
                            candle.range = self.market_type.round_price(candle.high - candle.low, self.tick_size, self.decimal_accuracy);
                            candle.volume += bar.volume;
                            candle.bid_volume += bar.bid_volume;
                            candle.ask_volume += bar.ask_volume;
                            candle.close = self.market_type.round_price((candle.open + candle.high + candle.low + candle.close) / dec!(4.0), self.tick_size, self.decimal_accuracy);
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        BaseDataEnum::Quote(quote) => {
                            candle.high = candle.high.max(quote.bid);
                            candle.low = candle.low.min(quote.bid);
                            candle.ask_volume += quote.ask_volume;
                            candle.bid_volume += quote.bid_volume;
                            candle.volume += quote.bid_volume + quote.ask_volume;
                            candle.range = self.market_type.round_price(candle.high - candle.low, self.tick_size, self.decimal_accuracy);
                            candle.close = self.market_type.round_price((candle.open + candle.high + candle.low + candle.close) / dec!(4.0), self.tick_size, self.decimal_accuracy);
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        _ => panic!(
                            "Invalid base data type for Heikin Ashi consolidator: {}",
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
}
