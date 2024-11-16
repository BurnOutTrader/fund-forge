use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal};
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Elder Ray Index
/// Combines trend-following and momentum analysis to evaluate buying and selling pressure.
///
/// # Plots
/// - "bull_power": The strength of buying pressure.
/// - "bear_power": The strength of selling pressure.
/// - "ema": The trend direction determined by EMA.
///
/// # Parameters
/// - period: Lookback period for the EMA calculation.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Provides insights into market trends and pressure.
/// Positive Bull Power and negative Bear Power indicate a strong upward trend.
#[derive(Clone, Debug)]
pub struct ElderRayIndex {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: u64,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color_bull: Color,
    plot_color_bear: Color,
    plot_color_ema: Color,
    ema: Option<Decimal>,
    multiplier: Decimal,
}

impl Display for ElderRayIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl ElderRayIndex {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        plot_color_bull: Color,
        plot_color_bear: Color,
        plot_color_ema: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        let multiplier = Decimal::from(2) / (Decimal::from(period) + Decimal::from(1));

        Box::new(ElderRayIndex {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            tick_size,
            plot_color_bull,
            plot_color_bear,
            plot_color_ema,
            ema: None,
            multiplier,
        })
    }

    /// Extract high, low, and close prices from BaseDataEnum.
    /// Specific to Elder Ray Index.
    fn get_price(&self, data: &BaseDataEnum) -> Option<(Price, Price, Price)> {
        match data {
            BaseDataEnum::Candle(candle) => Some((candle.high, candle.low, candle.close)),
            BaseDataEnum::QuoteBar(bar) => Some((bar.bid_high, bar.bid_low, bar.bid_close)),
            _ => None,
        }
    }

    /// Apply tick rounding if enabled, otherwise round to decimal accuracy.
    fn round_value(&self, value: Decimal) -> Decimal {
        if self.tick_rounding {
            round_to_tick_size(value, self.tick_size)
        } else {
            value.round_dp(self.decimal_accuracy)
        }
    }

    /// Update the EMA using the current close price.
    fn update_ema(&mut self, close: Decimal) -> Decimal {
        match self.ema {
            Some(prev_ema) => {
                let ema = (close - prev_ema) * self.multiplier + prev_ema;
                self.ema = Some(ema);
                ema
            }
            None => {
                self.ema = Some(close); // Initialize EMA with the first close price
                close
            }
        }
    }
}

impl Indicators for ElderRayIndex {
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

        // Extract high, low, and close prices from the base data
        let (high, low, close) = self.get_price(base_data)?;
        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data is available
        if self.base_data_history.len() < self.period as usize {
            return None;
        }

        // Update the EMA
        let ema = self.update_ema(close);

        // Calculate Bull Power and Bear Power
        let bull_power = self.round_value(high - ema);
        let bear_power = self.round_value(low - ema);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "bull_power".to_string(),
            IndicatorPlot::new("Bull Power".to_string(), bull_power, self.plot_color_bull.clone()),
        );
        plots.insert(
            "bear_power".to_string(),
            IndicatorPlot::new("Bear Power".to_string(), bear_power, self.plot_color_bear.clone()),
        );
        plots.insert(
            "ema".to_string(),
            IndicatorPlot::new("EMA".to_string(), ema, self.plot_color_ema.clone()),
        );

        // Create and store indicator values
        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.time_closed_utc(),
        );

        self.history.add(values.clone());
        self.is_ready = true;
        Some(vec![values])
    }

    fn subscription(&self) -> &DataSubscription {
        &self.subscription
    }

    fn reset(&mut self) {
        self.history.clear();
        self.base_data_history.clear();
        self.ema = None;
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