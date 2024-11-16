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

/// Commodity Channel Index (CCI)
/// Measures current price level relative to average price level.
///
/// # Plots
/// - "cci": Main CCI line
/// - "zero_line": Zero reference line
/// - "overbought": Overbought level (+100)
/// - "oversold": Oversold level (-100)
/// - "signal": Market signal
/// - "strength": Trend strength
///
/// # Parameters
/// - period: Calculation period (typically 20)
/// - constant: Lambert constant (typically 0.015)
///
/// # Usage
/// Identifies cyclical trends and potential reversals.
#[derive(Clone, Debug)]
pub struct CommodityChannelIndex {
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
    cci_color: Color,
    zero_line_color: Color,
    ob_color: Color,     // Overbought line color
    os_color: Color,     // Oversold line color
    period: u64,
    constant: Decimal,   // Typically 0.015
    tick_rounding: bool,
    overbought_level: Decimal,  // Typically +100
    oversold_level: Decimal,    // Typically -100
}

impl Display for CommodityChannelIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl CommodityChannelIndex {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        constant: Decimal,
        cci_color: Color,
        zero_line_color: Color,
        ob_color: Color,
        os_color: Color,
        tick_rounding: bool,
        overbought_level: Decimal,
        oversold_level: Decimal,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let cci = CommodityChannelIndex {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            cci_color,
            zero_line_color,
            ob_color,
            os_color,
            period,
            constant,
            decimal_accuracy,
            tick_rounding,
            overbought_level,
            oversold_level,
        };
        cci
    }

    fn get_bar_data(data: &BaseDataEnum) -> Option<(Price, Price, Price)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_high,
                bar.bid_low,
                bar.bid_close,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.high,
                candle.low,
                candle.close,
            )),
            _ => None,
        }
    }

    fn calculate_typical_price(high: Decimal, low: Decimal, close: Decimal) -> Decimal {
        (high + low + close) / dec!(3.0)
    }

    fn calculate_mean_deviation(values: &[Decimal], sma: Decimal) -> Decimal {
        let sum: Decimal = values.iter()
            .map(|&x| (x - sma).abs())
            .sum();

        sum / Decimal::from(values.len())
    }

    fn round_value(&self, value: Decimal) -> Price {
        match self.tick_rounding {
            true => round_to_tick_size(value, self.tick_size),
            false => value.round_dp(self.decimal_accuracy),
        }
    }

    fn get_signal(&self, cci: Decimal, prev_cci: Option<Decimal>) -> String {
        match prev_cci {
            Some(prev) => {
                if cci > self.overbought_level && prev <= self.overbought_level {
                    "Overbought".to_string()
                } else if cci < self.oversold_level && prev >= self.oversold_level {
                    "Oversold".to_string()
                } else if cci > dec!(0.0) && prev <= dec!(0.0) {
                    "Bullish".to_string()
                } else if cci < dec!(0.0) && prev >= dec!(0.0) {
                    "Bearish".to_string()
                } else {
                    "Neutral".to_string()
                }
            },
            None => "Neutral".to_string()
        }
    }

    fn calculate_strength(&self, cci: Decimal) -> String {
        let abs_cci = cci.abs();
        if abs_cci > dec!(200.0) {
            "Extreme".to_string()
        } else if abs_cci > dec!(100.0) {
            "Strong".to_string()
        } else if abs_cci > dec!(50.0) {
            "Moderate".to_string()
        } else {
            "Weak".to_string()
        }
    }
}

impl Indicators for CommodityChannelIndex {
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

        let history = self.base_data_history.history();

        // Calculate typical prices
        let typical_prices: Vec<Decimal> = history.iter()
            .filter_map(|data| {
                Self::get_bar_data(data).map(|(high, low, close)| {
                    Self::calculate_typical_price(high, low, close)
                })
            })
            .collect();

        if typical_prices.is_empty() {
            return None;
        }

        // Calculate SMA of typical prices
        let sma: Decimal = typical_prices.iter().sum::<Decimal>() / Decimal::from(self.period);

        // Calculate Mean Deviation
        let mean_dev = Self::calculate_mean_deviation(&typical_prices, sma);

        // Calculate CCI
        let cci = if mean_dev != dec!(0.0) {
            (typical_prices.last().unwrap() - sma) / (self.constant * mean_dev)
        } else {
            dec!(0.0)
        };

        let cci = self.round_value(cci);

        // Get previous CCI for signal generation
        let prev_cci = self.history.last()
            .and_then(|values| values.plots.get("cci"))
            .map(|plot| plot.value);

        // Generate signal and strength
        let signal = self.get_signal(cci, prev_cci);
        let strength = self.calculate_strength(cci);

        // Create plots
        let mut plots = BTreeMap::new();

        // Main CCI line
        plots.insert(
            "cci".to_string(),
            IndicatorPlot::new("CCI".to_string(), cci, self.cci_color.clone()),
        );

        // Reference lines
        plots.insert(
            "zero_line".to_string(),
            IndicatorPlot::new("Zero".to_string(), dec!(0.0), self.zero_line_color.clone()),
        );
        plots.insert(
            "overbought".to_string(),
            IndicatorPlot::new("Overbought".to_string(), self.overbought_level, self.ob_color.clone()),
        );
        plots.insert(
            "oversold".to_string(),
            IndicatorPlot::new("Oversold".to_string(), self.oversold_level, self.os_color.clone()),
        );

        // Signal and strength indicators
        plots.insert(
            "signal".to_string(),
            IndicatorPlot::new(signal, cci, self.cci_color.clone()),
        );
        plots.insert(
            "strength".to_string(),
            IndicatorPlot::new(strength, cci, self.cci_color.clone()),
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