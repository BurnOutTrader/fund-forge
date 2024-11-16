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
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Chaikin Money Flow (CMF)
/// A volume-weighted measure that indicates the level of accumulation or distribution over a specified
/// period. Combines price action with volume to identify buying and selling pressure. CMF measures the
/// money flow volume over a specific time period, helping identify market trends and potential reversals.
///
/// # Calculation Method
/// 1. Money Flow Multiplier = ((Close - Low) - (High - Close)) / (High - Low)
/// 2. Money Flow Volume = Money Flow Multiplier × Volume
/// 3. CMF = Sum(Money Flow Volume) / Sum(Volume) over N periods
///
/// # Plots
/// - "cmf": Main CMF line ranging from -1 to +1
///   - Positive values indicate buying pressure (accumulation)
///   - Negative values indicate selling pressure (distribution)
/// - "zero_line": Reference line at zero
///   - Crossing above indicates shift to bullish
///   - Crossing below indicates shift to bearish
/// - "signal": Market condition indicator
///   - "Overbought": When CMF exceeds overbought threshold
///   - "Oversold": When CMF drops below oversold threshold
///   - "Bullish": When CMF is positive but below overbought
///   - "Bearish": When CMF is negative but above oversold
///   - "Neutral": When CMF is near zero
/// - "trend": Trend direction classification
///   - "Bullish": Sustained positive CMF
///   - "Bearish": Sustained negative CMF
///   - "Neutral": CMF oscillating around zero
///
/// # Parameters
/// - period: Number of periods for calculation (typically 20 or 21)
/// - overbought_level: Upper threshold (typically 0.25 or 0.30)
/// - oversold_level: Lower threshold (typically -0.25 or -0.30)
/// - tick_rounding: Whether to round values to tick size
///
/// # Key Signals
/// 1. Zero Line Crossovers
///    - Crossing above zero suggests potential uptrend
///    - Crossing below zero suggests potential downtrend
///
/// 2. Divergence
///    - Bullish: Price making lower lows while CMF makes higher lows
///    - Bearish: Price making higher highs while CMF makes lower highs
///
/// 3. Volume Confirmation
///    - High volume with positive CMF confirms uptrend
///    - High volume with negative CMF confirms downtrend
///
/// # Common Usage
/// - Trend Confirmation: Use with price action to confirm trend direction
/// - Volume Analysis: Identify whether price moves are supported by volume
/// - Divergence Trading: Spot potential reversals through price/CMF divergence
/// - Support/Resistance: CMF levels can act as support/resistance
///
/// # Interpretation
/// - Strong Bullish: CMF > overbought_level with increasing volume
/// - Strong Bearish: CMF < oversold_level with increasing volume
/// - Weak Trend: CMF near zero or diverging from price
/// - Potential Reversal: CMF diverging from price at extremes
///
/// # Best Practices
/// 1. Use in conjunction with other indicators for confirmation
/// 2. Look for volume confirmation of CMF signals
/// 3. Pay attention to divergences at market extremes
/// 4. Consider longer periods (20+) for more reliable signals
/// 5. Watch for sustained moves above/below zero line
///
/// # Considerations
/// - More reliable in trending markets than ranging markets
/// - Can generate false signals in low volume conditions
/// - Best used with additional confirmation indicators
/// - May lag in fast-moving markets due to period calculation
#[derive(Clone, Debug)]
pub struct ChaikinMoneyFlow {
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
    cmf_color: Color,
    zero_line_color: Color,
    period: u64,
    tick_rounding: bool,
    overbought_level: Decimal,
    oversold_level: Decimal,
}

impl Display for ChaikinMoneyFlow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl ChaikinMoneyFlow {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        cmf_color: Color,
        zero_line_color: Color,
        tick_rounding: bool,
        overbought_level: Decimal,
        oversold_level: Decimal,
    ) -> Box<Self> {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let cmf = ChaikinMoneyFlow {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            cmf_color,
            zero_line_color,
            period,
            decimal_accuracy,
            tick_rounding,
            overbought_level,
            oversold_level,
        };
        Box::new(cmf)
    }

    fn get_bar_data(data: &BaseDataEnum) -> Option<(Price, Price, Price, Price, Volume)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_high,
                bar.bid_low,
                bar.bid_open,
                bar.bid_close,
                bar.bid_volume,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.high,
                candle.low,
                candle.open,
                candle.close,
                Decimal::from(candle.volume),
            )),
            _ => None,
        }
    }

    fn calculate_money_flow_multiplier(
        high: Decimal,
        low: Decimal,
        close: Decimal
    ) -> Decimal {
        if high == low {
            return dec!(0.0);
        }

        ((close - low) - (high - close)) / (high - low)
    }

    fn calculate_money_flow_volume(
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal
    ) -> Decimal {
        let multiplier = Self::calculate_money_flow_multiplier(high, low, close);
        multiplier * volume
    }

    fn calculate_cmf(&self, history: &Vec<BaseDataEnum>) -> Price {
        let mut sum_money_flow_volume = dec!(0.0);
        let mut sum_volume = dec!(0.0);

        for data in history {
            if let Some((high, low, _, close, volume)) = Self::get_bar_data(data) {
                sum_money_flow_volume += Self::calculate_money_flow_volume(high, low, close, volume);
                sum_volume += volume;
            }
        }

        if sum_volume == dec!(0.0) {
            return dec!(0.0);
        }

        let cmf = sum_money_flow_volume / sum_volume;

        match self.tick_rounding {
            true => round_to_tick_size(cmf, self.tick_size),
            false => cmf.round_dp(self.decimal_accuracy),
        }
    }

    fn get_signal(&self, cmf: Decimal) -> String {
        if cmf >= self.overbought_level {
            "Overbought".to_string()
        } else if cmf <= self.oversold_level {
            "Oversold".to_string()
        } else {
            "Neutral".to_string()
        }
    }

    fn get_trend(&self, cmf: Decimal) -> String {
        if cmf > dec!(0.0) {
            "Bullish".to_string()
        } else if cmf < dec!(0.0) {
            "Bearish".to_string()
        } else {
            "Neutral".to_string()
        }
    }
}

impl Indicators for ChaikinMoneyFlow {
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


        // Calculate CMF
        let cmf = self.calculate_cmf(&self.base_data_history.history);

        // Create plots
        let mut plots = BTreeMap::new();

        // Main CMF line
        plots.insert(
            "cmf".to_string(),
            IndicatorPlot::new("CMF".to_string(), cmf, self.cmf_color.clone()),
        );

        // Zero line (for reference)
        plots.insert(
            "zero_line".to_string(),
            IndicatorPlot::new("Zero Line".to_string(), dec!(0.0), self.zero_line_color.clone()),
        );

        // Overbought level
        plots.insert(
            "overbought".to_string(),
            IndicatorPlot::new("Overbought".to_string(), self.overbought_level, self.zero_line_color.clone()),
        );

        // Oversold level
        plots.insert(
            "oversold".to_string(),
            IndicatorPlot::new("Oversold".to_string(), self.oversold_level, self.zero_line_color.clone()),
        );

        // Signal and trend
        let signal = self.get_signal(cmf);
        let trend = self.get_trend(cmf);

        plots.insert(
            "signal".to_string(),
            IndicatorPlot::new(signal, cmf, self.cmf_color.clone()),
        );

        plots.insert(
            "trend".to_string(),
            IndicatorPlot::new(trend, cmf, self.cmf_color.clone()),
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
