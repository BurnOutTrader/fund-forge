use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::helpers::converters::open_time;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use chrono::{DateTime, Utc};
use crate::consolidators::consolidator_enum::ConsolidatedData;

pub struct HeikinAshiConsolidator {
    current_data: Option<BaseDataEnum>,
    pub(crate) subscription: DataSubscription,
    pub(crate) history: RollingWindow<BaseDataEnum>,
    previous_ha_close: f64,
    previous_ha_open: f64,
    tick_size: f64,
}

impl HeikinAshiConsolidator {
    fn candle_from_base_data(
        &self,
        ha_open: f64,
        ha_high: f64,
        ha_low: f64,
        ha_close: f64,
        volume: f64,
        time: String,
        is_closed: bool,
        range: f64,
    ) -> Candle {
        Candle {
            symbol: self.subscription.symbol.clone(),
            open: ha_open,
            high: ha_high,
            low: ha_low,
            close: ha_close,
            volume,
            time,
            resolution: self.subscription.resolution.clone(),
            is_closed,
            range,
            candle_type: CandleType::HeikinAshi,
        }
    }

    fn new_heikin_ashi_candle(&mut self, new_data: &BaseDataEnum) -> Candle {
        match new_data {
            BaseDataEnum::Candle(candle) => {
                if self.previous_ha_close == 0.0 && self.previous_ha_open == 0.0 {
                    self.previous_ha_close = candle.close;
                    self.previous_ha_open = candle.open;
                }
                let ha_close = round_to_tick_size(
                    (candle.open + candle.high + candle.low + candle.close) / 4.0,
                    self.tick_size,
                );
                let ha_open = round_to_tick_size(
                    (self.previous_ha_open + self.previous_ha_close) / 2.0,
                    self.tick_size,
                );
                let ha_high = candle.high.max(ha_open).max(ha_close);
                let ha_low = candle.low.min(ha_open).min(ha_close);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;
                let time = open_time(&self.subscription, new_data.time_utc());

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    candle.volume,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            BaseDataEnum::Price(price) => {
                if self.previous_ha_close == 0.0 && self.previous_ha_open == 0.0 {
                    self.previous_ha_close = price.price;
                    self.previous_ha_open = price.price;
                }
                let ha_close = price.price;
                let ha_open = round_to_tick_size(
                    (self.previous_ha_open + self.previous_ha_close) / 2.0,
                    self.tick_size,
                );
                let ha_high = ha_close.max(ha_open);
                let ha_low = ha_close.min(ha_open);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;
                let time = open_time(&self.subscription, new_data.time_utc());

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    0.0,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            BaseDataEnum::QuoteBar(bar) => {
                if self.previous_ha_close == 0.0 && self.previous_ha_open == 0.0 {
                    self.previous_ha_close = bar.bid_close;
                    self.previous_ha_open = bar.bid_close;
                }
                let ha_close = bar.bid_close;
                let ha_open = round_to_tick_size(
                    (self.previous_ha_open + self.previous_ha_close) / 2.0,
                    self.tick_size,
                );
                let ha_high = ha_close.max(ha_open);
                let ha_low = ha_close.min(ha_open);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;
                let time = open_time(&self.subscription, new_data.time_utc());

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    bar.volume,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            BaseDataEnum::Tick(tick) => {
                if self.previous_ha_close == 0.0 && self.previous_ha_open == 0.0 {
                    self.previous_ha_close = tick.price;
                    self.previous_ha_open = tick.price;
                }
                let ha_close = tick.price;
                let ha_open = round_to_tick_size(
                    (self.previous_ha_open + self.previous_ha_close) / 2.0,
                    self.tick_size,
                );
                let ha_high = ha_close.max(ha_open);
                let ha_low = ha_close.min(ha_open);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;
                let time = open_time(&self.subscription, new_data.time_utc());

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    tick.volume,
                    time.to_string(),
                    false,
                    ha_high - ha_low,
                )
            }
            BaseDataEnum::Quote(quote) => {
                if self.previous_ha_close == 0.0 && self.previous_ha_open == 0.0 {
                    self.previous_ha_close = quote.bid;
                    self.previous_ha_open = quote.bid;
                }
                let ha_close = quote.bid;
                let ha_open = round_to_tick_size(
                    (self.previous_ha_open + self.previous_ha_close) / 2.0,
                    self.tick_size,
                );
                let ha_high = ha_close.max(ha_open);
                let ha_low = ha_close.min(ha_open);

                // Update previous Heikin Ashi values for next bar
                self.previous_ha_close = ha_close;
                self.previous_ha_open = ha_open;
                let time = open_time(&self.subscription, new_data.time_utc());

                self.candle_from_base_data(
                    ha_open,
                    ha_high,
                    ha_low,
                    ha_close,
                    0.0,
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
        history_to_retain: u64,
    ) -> Result<HeikinAshiConsolidator, String> {
        if subscription.base_data_type == BaseDataType::Fundamentals {
            return Err(format!(
                    "{} is an Invalid base data type for HeikinAshiConsolidator",
                    subscription.base_data_type
                ),
            );
        }

        if let Some(candle_type) = &subscription.candle_type {
            if candle_type != &CandleType::HeikinAshi {
                return Err(format!(
                        "{:?} is an Invalid candle type for HeikinAshiConsolidator",
                        candle_type
                    ),
                );
            }
        }
        let tick_size = match subscription
            .symbol
            .data_vendor
            .tick_size(subscription.symbol.clone())
            .await
        {
            Ok(size) => size,
            Err(e) => {
                return Err(format!("Error getting tick size: {}", e),
                )
            }
        };

        Ok(HeikinAshiConsolidator {
            current_data: None,
            subscription,
            history: RollingWindow::new(history_to_retain),
            previous_ha_close: 0.0,
            previous_ha_open: 0.0,
            tick_size,
        })
    }

    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        if let Some(current_data) = self.current_data.as_mut() {
            if time >= current_data.time_created_utc() {
                let mut return_data = current_data.clone();
                return_data.set_is_closed(true);
                self.current_data = None;
                return Some(return_data);
            }
        }
        None
    }

    //problem where this is returning a closed candle constantly
    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        if self.current_data.is_none() {
            let data = self.new_heikin_ashi_candle(base_data);
            self.current_data = Some(BaseDataEnum::Candle(data));
            return ConsolidatedData::with_open(self.current_data.clone().unwrap())
        } else if let Some(current_bar) = self.current_data.as_mut() {
            if base_data.time_created_utc() >= current_bar.time_created_utc() {
                let mut consolidated_bar = current_bar.clone();
                consolidated_bar.set_is_closed(true);
                self.history.add(consolidated_bar.clone());
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
                            candle.close = tick.price;
                            candle.range =
                                round_to_tick_size(candle.high - candle.low, self.tick_size.clone());
                            candle.volume += tick.volume;
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        BaseDataEnum::Candle(new_candle) => {
                            candle.high = new_candle.high.max(candle.high);
                            candle.low = new_candle.low.min(candle.low);
                            candle.close = new_candle.close;
                            candle.range =
                                round_to_tick_size(candle.high - candle.low, self.tick_size.clone());
                            candle.volume += new_candle.volume;
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        BaseDataEnum::Price(price) => {
                            candle.high = price.price.max(candle.high);
                            candle.low = price.price.min(candle.low);
                            candle.close = price.price;
                            candle.range =
                                round_to_tick_size(candle.high - candle.low, self.tick_size.clone());
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        BaseDataEnum::QuoteBar(bar) => {
                            candle.high = bar.bid_high.max(candle.high);
                            candle.low = bar.bid_low.min(candle.low);
                            candle.close = bar.bid_close;
                            candle.range =
                                round_to_tick_size(candle.high - candle.low, self.tick_size.clone());
                            candle.volume += bar.volume;
                            return ConsolidatedData::with_open(BaseDataEnum::Candle(candle.clone()))
                        }
                        BaseDataEnum::Quote(quote) => {
                            candle.high = candle.high.max(quote.bid);
                            candle.low = candle.low.min(quote.bid);
                            candle.close = quote.bid;
                            candle.range =
                                round_to_tick_size(candle.high - candle.low, self.tick_size.clone());
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

    pub(crate) fn history(&self) -> RollingWindow<BaseDataEnum> {
        self.history.clone()
    }

    pub(crate) fn index(&self, index: u64) -> Option<BaseDataEnum> {
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
