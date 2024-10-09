use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use crate::gui_types::settings::Color;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::{DataSubscription};
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};
use rust_decimal_macros::dec;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::traits::BaseData;

/// Renko Indicator
/// The Renko Indicator can output more than 1 "IndicatorValues" object per update, multiple blocks may be returned in a single buffer.
/// `plots: "open", "close"`
#[derive(Clone, Debug)]
pub struct Renko {
    name: IndicatorName,
    pub(crate) subscription: DataSubscription,
    market_type: MarketType,
    decimal_accuracy: u32,
    tick_size: Decimal,
    up_color: Color,
    down_color: Color,
    history: RollingWindow<IndicatorValues>,
    is_ready: bool,
    renko_range: Decimal,
    open_price: Option<Decimal>,
    open_time: Option<DateTime<Utc>>
}

impl Renko {
    #[allow(dead_code)]
    pub(crate) async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        renko_range: Decimal,
        up_color: Color,
        down_color: Color,
        history_to_retain: usize,
    ) -> Self {
        if subscription.base_data_type != BaseDataType::Quotes && subscription.base_data_type != BaseDataType::Ticks {
            panic!("Incorrect BaseDataType for Renko Subscription")
        }
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone()).await.unwrap();

        Renko {
            name,
            market_type: subscription.market_type.clone(),
            subscription,
            decimal_accuracy,
            renko_range,
            tick_size,
            up_color,
            down_color,
            history: RollingWindow::new(history_to_retain),
            is_ready: false,
            open_price: None,
            open_time: None,
        }
    }

    fn process_price(&mut self, price: Decimal, time: DateTime<Utc>) -> Option<Vec<IndicatorValues>> {
        if self.open_price.is_none() {
            self.open_price = Some(price);
            self.open_time = Some(time);
            return None;
        }

        let open_price = self.open_price.unwrap();
        let mut blocks = Vec::new();

        let price_diff = price - open_price;
        let blocks_to_create = (price_diff / self.renko_range).abs().floor();

        if blocks_to_create >= dec!(1) {
            let direction = if price_diff > dec!(0) { 1 } else { -1 };

            for i in 0..blocks_to_create.to_i64().unwrap() {
                let new_price = self.market_type.round_price(open_price + (self.renko_range * Decimal::from(direction * (i + 1))), self.tick_size, self.decimal_accuracy);
                let block = self.create_renko_block(open_price, new_price, self.open_time.unwrap());
                blocks.push(block.clone());
                self.history.add(block);

                self.open_price = Some(new_price);
                self.open_time = Some(time);
            }

            self.is_ready = true;
            Some(blocks)
        } else {
            None
        }
    }

    fn create_renko_block(&self, open: Decimal, close: Decimal, time: DateTime<Utc>) -> IndicatorValues {
        let mut values = IndicatorValues::new(self.name.clone(), self.subscription.clone(), BTreeMap::new(), time);

        let color = if close > open { self.up_color.clone() } else { self.down_color.clone() };

        let open_plot = IndicatorPlot::new("open".to_string(), open, color.clone());
        let close_plot = IndicatorPlot::new("close".to_string(), close, color);

        values.insert_plot("open".to_string(), open_plot);
        values.insert_plot("close".to_string(), close_plot);

        values
    }
}

impl Indicators for Renko {
    fn name(&self) -> IndicatorName {
        self.name.clone()
    }

    fn history_to_retain(&self) -> usize {
        self.history.len()
    }

    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<Vec<IndicatorValues>> {
        if base_data.subscription() != self.subscription {
            return None;
        }

        match base_data {
            BaseDataEnum::Tick(tick) => self.process_price(tick.price, tick.time_utc()),
            BaseDataEnum::Quote(quote) => self.process_price(quote.bid, quote.time_utc()),
            _ => None,
        }
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
    }

    fn index(&self, index: usize) -> Option<IndicatorValues> {
        if !self.is_ready {
            return None;
        }
        self.history.get(index).cloned()
    }

    fn current(&self) -> Option<IndicatorValues> {
        if !self.is_ready {
            return None;
        }
        self.history.last().cloned()
    }

    fn plots(&self) -> RollingWindow<IndicatorValues> {
        self.history.clone()
    }

    fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn history(&self) -> RollingWindow<IndicatorValues> {
        self.history.clone()
    }

    fn data_required_warmup(&self) -> u64 {
        self.history.len() as u64 * 100
    }
}
