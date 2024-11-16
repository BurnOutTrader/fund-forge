use std::collections::BTreeMap;
use rust_decimal::{Decimal};
use rust_decimal::prelude::Zero;
use crate::gui_types::settings::Color;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::new_types::Price;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::strategies::indicators::indicator_values::{IndicatorPlot, IndicatorValues};
use crate::strategies::indicators::indicators_trait::{IndicatorName, Indicators};

/// Williams VIX Fix
/// A volatility-based indicator inspired by the VIX, identifying market tops and bottoms.
///
/// # Plots
/// - "vix_fix": The main VIX Fix line.
///
/// # Parameters
/// - period: Lookback period for the lowest low.
/// - smoothing: Smoothing period for the VIX Fix line.
///
/// # Usage
/// Identifies tops and bottoms using volatility spikes, suitable for mean reversion and volatility trading.
#[derive(Clone, Debug)]
pub struct WilliamsVIXFix {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    base_data_history: RollingWindow<BaseDataEnum>,
    is_ready: bool,
    period: usize,
    smoothing: usize,
    plot_color: Color,
}

impl WilliamsVIXFix {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: usize,
        smoothing: usize,
        plot_color: Color,
    ) -> Box<Self> {
        Box::new(WilliamsVIXFix {
            name,
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period + smoothing),
            is_ready: false,
            period,
            smoothing,
            plot_color,
        })
    }

    /// Extract high, low, and close prices from BaseDataEnum.
    fn get_prices(&self, data: &BaseDataEnum) -> Option<(Price, Price)> {
        match data {
            BaseDataEnum::Candle(candle) => Some((candle.close, candle.low)),
            BaseDataEnum::QuoteBar(bar) => Some((bar.bid_close, bar.bid_low)),
            _ => None,
        }
    }

    /// Calculate the VIX Fix.
    fn calculate_vix_fix(&self, closes: &[Decimal], lows: &[Decimal]) -> Decimal {
        let lowest_low = lows.iter().copied().take(self.period).min().unwrap_or(Decimal::zero());
        let close = closes.last().cloned().unwrap_or(Decimal::zero());
        let raw_vix = (close - lowest_low) / lowest_low * Decimal::from(100);
        raw_vix
    }
}

impl Indicators for WilliamsVIXFix {
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

        if self.base_data_history.len() < self.period + self.smoothing {
            return None;
        }

        let closes: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_prices(data).map(|(c, _)| c))
            .collect();

        let lows: Vec<Decimal> = self
            .base_data_history
            .history()
            .iter()
            .filter_map(|data| self.get_prices(data).map(|(_, l)| l))
            .collect();

        let raw_vix = self.calculate_vix_fix(&closes, &lows);
        let smoothed_vix = raw_vix; // Smoothing logic if required

        let mut plots = BTreeMap::new();
        plots.insert(
            "vix_fix".to_string(),
            IndicatorPlot::new("VIX Fix".to_string(), smoothed_vix, self.plot_color.clone()),
        );

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
        (self.period + self.smoothing) as u64
    }
}