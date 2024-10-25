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
    pub async fn new(
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
        // Initialize if needed
        if self.open_price.is_none() {
            self.open_price = Some(price);
            self.open_time = Some(time);
            return None;
        }

        let open_price = self.open_price.unwrap();
        let mut blocks = Vec::new();

        // Calculate movement needed to complete a block
        let distance_moved = price - open_price;
        if distance_moved.abs() >= self.renko_range {
            // Calculate how many full blocks can be created
            let num_blocks = (distance_moved / self.renko_range).abs().floor();
            let direction = if distance_moved > dec!(0) { 1 } else { -1 };

            // Only proceed if we have at least one full block
            if num_blocks >= dec!(1) {
                for i in 0..num_blocks.to_i64().unwrap() {
                    let block_close = self.market_type.round_price(
                        open_price + (self.renko_range * Decimal::from(direction) * Decimal::from(i + 1)),
                        self.tick_size,
                        self.decimal_accuracy
                    );

                    let block = self.create_renko_block(
                        open_price + (self.renko_range * Decimal::from(direction) * Decimal::from(i)),
                        block_close,
                        self.open_time.unwrap()
                    );

                    blocks.push(block);
                }

                // Update open price to last block's close
                if let Some(last_block) = blocks.last() {
                    self.open_price = last_block.get_plot(&"close".to_string()).map(|plot| plot.value);
                    self.open_time = Some(time);
                }

                self.is_ready = true;
         /*       println!("Created {} Renko blocks from {} to {}",
                         blocks.len(), open_price, self.open_price.unwrap());*/
                return Some(blocks);
            }
        }

        None
    }

    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<Vec<IndicatorValues>> {
        if base_data.subscription() != self.subscription {
            return None;
        }

        match base_data {
            BaseDataEnum::Tick(tick) => {
                // Only process if movement >= tick size to reduce noise
                if let Some(last_price) = self.open_price {
                    if (tick.price - last_price).abs() < self.tick_size {
                        return None;
                    }
                }
                self.process_price(tick.price, tick.time_utc())
            },
            BaseDataEnum::Quote(quote) => {
                // Same check for quotes
                if let Some(last_price) = self.open_price {
                    if (quote.bid - last_price).abs() < self.tick_size {
                        return None;
                    }
                }
                self.process_price(quote.bid, quote.time_utc())
            },
            _ => None,
        }
    }

    fn create_renko_block(&self, open: Decimal, close: Decimal, time: DateTime<Utc>) -> IndicatorValues {
        let mut values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            BTreeMap::new(),
            time
        );

        let color = if close > open { self.up_color.clone() } else { self.down_color.clone() };

        // Only create if it's a full block
        let movement = (close - open).abs();
        if movement >= self.renko_range {
            let open_plot = IndicatorPlot::new("open".to_string(), open, color.clone());
            let close_plot = IndicatorPlot::new("close".to_string(), close, color);

            values.insert_plot("open".to_string(), open_plot);
            values.insert_plot("close".to_string(), close_plot);
        }

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

        let (price, time) = match base_data {
            BaseDataEnum::Tick(tick) => (tick.price, tick.time_utc()),
            BaseDataEnum::Quote(quote) => (quote.bid, quote.time_utc()),
            _ => return None,
        };

        // Only process if price actually changed
        if let Some(last_price) = self.open_price {
            if (price - last_price).abs() < self.tick_size {
                return None;
            }
        }

        self.process_price(price, time)
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
