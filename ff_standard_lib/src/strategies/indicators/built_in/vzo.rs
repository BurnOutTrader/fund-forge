use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal};
use rust_decimal::prelude::Zero;
use rust_decimal_macros::dec;
use crate::gui_types::settings::Color;
use crate::helpers::decimal_calculators::round_to_tick_size;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::{Price, Volume};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Volume Zone Oscillator (VZO)
/// A volume-based oscillator that measures buying and selling pressure.
///
/// # Plots
/// - "vzo": The main VZO line.
///
/// # Parameters
/// - period: Lookback period for volume analysis.
/// - tick_rounding: Whether to round values to tick size.
///
/// # Usage
/// Helps identify buying and selling pressure in the market.
/// Values >40 indicate strong buying pressure, while values <-40 indicate strong selling pressure.
#[derive(Clone, Debug)]
pub struct VolumeZoneOscillator {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    decimal_accuracy: u32,
    is_ready: bool,
    period: u64,
    tick_rounding: bool,
    tick_size: Decimal,
    plot_color: Color,
    positive_volume: Decimal,
    total_volume: Decimal,
}

impl Display for VolumeZoneOscillator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl VolumeZoneOscillator {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        plot_color: Color,
        tick_rounding: bool,
    ) -> Box<Self> {
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(subscription.symbol.name.clone())
            .await
            .unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(subscription.symbol.name.clone())
            .await
            .unwrap();

        Box::new(VolumeZoneOscillator {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            decimal_accuracy,
            is_ready: false,
            period,
            tick_rounding,
            tick_size,
            plot_color,
            positive_volume: Decimal::zero(),
            total_volume: Decimal::zero(),
        })
    }

    /// Extract price and volume from BaseDataEnum.
    /// Specific to Volume Zone Oscillator.
    fn get_price_and_volume(&self, data: &BaseDataEnum) -> Option<(Price, Volume)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((bar.bid_close, bar.bid_volume)),
            BaseDataEnum::Candle(candle) => Some((candle.close, Decimal::from(candle.volume))),
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

    /// Calculate the Volume Zone Oscillator (VZO).
    fn calculate_vzo(&self, positive_volume: Decimal, total_volume: Decimal) -> Decimal {
        if total_volume == Decimal::zero() {
            Decimal::zero()
        } else {
            (positive_volume / total_volume) * dec!(100.0)
        }
    }
}

impl Indicators for VolumeZoneOscillator {
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

        // Extract price and volume from the base data
        let (close, volume) = self.get_price_and_volume(base_data)?;
        self.base_data_history.add(base_data.clone());

        // Ensure sufficient data is available
        if self.base_data_history.len() < self.period as usize {
            return None;
        }

        // Calculate price change
        let previous_price = self
            .base_data_history
            .get(self.base_data_history.len() - 2)
            .and_then(|data| self.get_price_and_volume(data).map(|(price, _)| price))
            .unwrap_or(close);

        // Update positive and total volume
        if close > previous_price {
            self.positive_volume += volume;
        }
        self.total_volume += volume;

        // Calculate VZO
        let vzo = self.round_value(self.calculate_vzo(self.positive_volume, self.total_volume));

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "vzo".to_string(),
            IndicatorPlot::new("Volume Zone Oscillator".to_string(), vzo, self.plot_color.clone()),
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
        self.positive_volume = Decimal::zero();
        self.total_volume = Decimal::zero();
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