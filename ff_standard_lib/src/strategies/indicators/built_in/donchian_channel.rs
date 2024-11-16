use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::product_maps::rithmic::maps::extract_symbol_from_contract;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::enums::MarketType;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};


#[derive(Clone, Debug)]
pub struct DonchianChannels {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    #[allow(unused)]
    market_type: MarketType,
    #[allow(unused)]
    tick_size: Decimal,
    decimal_accuracy: u32,
    is_ready: bool,
    upper_color: Color,
    middle_color: Color,
    lower_color: Color,
    period: u64,
    tick_rounding: bool,
    breakout_level: Option<Decimal>,  // For tracking new highs/lows
    last_trend: Option<bool>,         // true for up, false for down
}

impl Display for DonchianChannels {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl DonchianChannels {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        upper_color: Color,
        middle_color: Color,
        lower_color: Color,
        tick_rounding: bool,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let donchian = DonchianChannels {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            upper_color,
            middle_color,
            lower_color,
            period,
            decimal_accuracy,
            tick_rounding,
            breakout_level: None,
            last_trend: None,
        };
        donchian
    }

    fn get_price_data(data: &BaseDataEnum) -> Option<(Price, Price)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_high,
                bar.bid_low,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.high,
                candle.low,
            )),
            _ => None,
        }
    }

    fn round_value(&self, value: Decimal) -> Price {
        match self.tick_rounding {
            true => round_to_tick_size(value, self.tick_size),
            false => value.round_dp(self.decimal_accuracy),
        }
    }

    fn detect_breakout(&mut self, upper: Decimal, lower: Decimal) -> Option<bool> {
        match self.breakout_level {
            None => {
                self.breakout_level = Some(upper);
                None
            },
            Some(level) => {
                if upper > level {
                    self.breakout_level = Some(upper);
                    Some(true)  // Upward breakout
                } else if lower < level {
                    self.breakout_level = Some(lower);
                    Some(false) // Downward breakout
                } else {
                    None
                }
            }
        }
    }

    fn calculate_channels(&self) -> (Price, Price, Price) {
        let history = self.base_data_history.history();

        let mut highest_high = dec!(-999999.0);
        let mut lowest_low = dec!(999999.0);

        // Find highest high and lowest low over the period
        for data in history.iter() {
            if let Some((high, low)) = Self::get_price_data(data) {
                highest_high = highest_high.max(high);
                lowest_low = lowest_low.min(low);
            }
        }

        let middle = (highest_high + lowest_low) / dec!(2.0);

        (
            self.round_value(highest_high),
            self.round_value(middle),
            self.round_value(lowest_low),
        )
    }

    fn calculate_channel_stats(&self, upper: Decimal, lower: Decimal) -> (Price, Price) {
        let channel_width = upper - lower;
        let channel_percent = channel_width / lower * dec!(100.0);

        (
            self.round_value(channel_width),
            self.round_value(channel_percent),
        )
    }
}

impl Indicators for DonchianChannels {
    fn name(&self) -> IndicatorName {
        self.name.clone()
    }

    fn history_to_retain(&self) -> usize {
        self.history.number.clone() as usize
    }

    fn update_base_data(&mut self, base_data: &BaseDataEnum) -> Option<Vec<IndicatorValues>> {
        if !base_data.is_closed() {
            return None;
        }

        self.base_data_history.add(base_data.clone());

        if !self.is_ready {
            if !self.base_data_history.is_full() {
                return None;
            }
            self.is_ready = true;
        }

        // Calculate channel levels
        let (upper, middle, lower) = self.calculate_channels();

        // Calculate additional statistics
        let (width, percent_width) = self.calculate_channel_stats(upper, lower);

        // Detect breakouts
        if let Some(breakout_up) = self.detect_breakout(upper, lower) {
            self.last_trend = Some(breakout_up);
        }

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "upper".to_string(),
            IndicatorPlot::new("Upper".to_string(), upper, self.upper_color.clone()),
        );
        plots.insert(
            "middle".to_string(),
            IndicatorPlot::new("Middle".to_string(), middle, self.middle_color.clone()),
        );
        plots.insert(
            "lower".to_string(),
            IndicatorPlot::new("Lower".to_string(), lower, self.lower_color.clone()),
        );
        plots.insert(
            "width".to_string(),
            IndicatorPlot::new("Width".to_string(), width, self.middle_color.clone()),
        );
        plots.insert(
            "percent_width".to_string(),
            IndicatorPlot::new("% Width".to_string(), percent_width, self.middle_color.clone()),
        );

        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.time_closed_utc(),
        );

        self.history.add(values.clone());
        Some(vec![values])
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
        self.is_ready = false;
        self.breakout_level = None;
        self.last_trend = None;
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
        self.period
    }
}
