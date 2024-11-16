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
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;
use chrono::{DateTime, Datelike, Utc};
use crate::standardized_types::market_hours::TradingHours;

#[derive(Clone, Debug)]
pub struct VolumeWeightedAveragePrice {
    name: IndicatorName,
    subscription: DataSubscription,
    history: RollingWindow<IndicatorValues>,
    #[allow(unused)]
    market_type: MarketType,
    #[allow(unused)]
    tick_size: Decimal,
    decimal_accuracy: u32,
    is_ready: bool,
    vwap_color: Color,
    upper_band_color: Color,
    lower_band_color: Color,
    tick_rounding: bool,
    cumulative_pv: Decimal,
    cumulative_volume: Decimal,
    std_dev_multiplier: Decimal,
    last_reset_day: Option<u32>,
    squared_diff_sum: Decimal,
    trading_hours: TradingHours,
    was_last_bar_in_session: bool,
}

impl Display for VolumeWeightedAveragePrice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl VolumeWeightedAveragePrice {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        std_dev_multiplier: Decimal,
        vwap_color: Color,
        upper_band_color: Color,
        lower_band_color: Color,
        tick_rounding: bool,
        trading_hours: TradingHours,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let vwap = VolumeWeightedAveragePrice {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            is_ready: false,
            tick_size,
            vwap_color,
            upper_band_color,
            lower_band_color,
            decimal_accuracy,
            tick_rounding,
            cumulative_pv: dec!(0.0),
            cumulative_volume: dec!(0.0),
            std_dev_multiplier,
            last_reset_day: None,
            squared_diff_sum: dec!(0.0),
            trading_hours,
            was_last_bar_in_session: false,
        };
        vwap
    }

    fn get_typical_price(data: &BaseDataEnum) -> Option<(Price, Volume)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => {
                let typical_price = (bar.bid_high + bar.bid_low + bar.bid_close) / dec!(3.0);
                Some((typical_price, bar.bid_volume))
            },
            BaseDataEnum::Candle(candle) => {
                let typical_price = (candle.high + candle.low + candle.close) / dec!(3.0);
                Some((typical_price, Decimal::from(candle.volume)))
            },
            _ => None,
        }
    }

    fn should_reset(&mut self, time: DateTime<Utc>) -> bool {
        // Reset if:
        // 1. We're in a new session after being out of session
        // 2. We're on a new day and before/at session start
        let is_in_session = self.trading_hours.is_market_open(time);

        if !is_in_session {
            self.was_last_bar_in_session = false;
            return false;
        }

        if !self.was_last_bar_in_session {
            return true;
        }

        let market_time = time.with_timezone(&self.trading_hours.timezone);
        match self.last_reset_day {
            None => true,
            Some(last_day) => {
                let current_day = market_time.day();
                current_day != last_day
            }
        }
    }

    fn calculate_vwap(&self) -> Price {
        if self.cumulative_volume == dec!(0.0) {
            return dec!(0.0);
        }

        let vwap = self.cumulative_pv / self.cumulative_volume;

        match self.tick_rounding {
            true => round_to_tick_size(vwap, self.tick_size),
            false => vwap.round_dp(self.decimal_accuracy),
        }
    }

    fn calculate_bands(&self, vwap: Decimal) -> Option<(Price, Price)> {
        if self.cumulative_volume <= dec!(1.0) {
            return None;
        }

        let variance = self.squared_diff_sum / self.cumulative_volume;
        let std_dev = variance.sqrt()?;
        let band_width = std_dev * self.std_dev_multiplier;

        let upper = match self.tick_rounding {
            true => round_to_tick_size(vwap + band_width, self.tick_size),
            false => (vwap + band_width).round_dp(self.decimal_accuracy),
        };

        let lower = match self.tick_rounding {
            true => round_to_tick_size(vwap - band_width, self.tick_size),
            false => (vwap - band_width).round_dp(self.decimal_accuracy),
        };

        Some((upper, lower))
    }
}

impl Indicators for VolumeWeightedAveragePrice {
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

        let current_time = base_data.time_closed_utc();

        // Only process if we're in market hours
        if !self.trading_hours.is_market_open(current_time) {
            self.was_last_bar_in_session = false;
            return None;
        }

        // Check for reset conditions
        if self.should_reset(current_time) {
            self.cumulative_pv = dec!(0.0);
            self.cumulative_volume = dec!(0.0);
            self.squared_diff_sum = dec!(0.0);
            self.last_reset_day = Some(current_time
                .with_timezone(&self.trading_hours.timezone)
                .day());
        }

        self.was_last_bar_in_session = true;

        // Get price and volume data
        let (typical_price, volume) = Self::get_typical_price(base_data)?;

        // Update cumulative values
        self.cumulative_pv += typical_price * volume;
        self.cumulative_volume += volume;

        // Calculate VWAP
        let vwap = self.calculate_vwap();
        if vwap == dec!(0.0) {
            return None;
        }

        // Update standard deviation calculation
        let price_diff = typical_price - vwap;
        self.squared_diff_sum += price_diff * price_diff * volume;

        // Calculate bands
        let bands = self.calculate_bands(vwap);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "vwap".to_string(),
            IndicatorPlot::new("VWAP".to_string(), vwap, self.vwap_color.clone()),
        );

        if let Some((upper, lower)) = bands {
            plots.insert(
                "upper_band".to_string(),
                IndicatorPlot::new("Upper Band".to_string(), upper, self.upper_band_color.clone()),
            );
            plots.insert(
                "lower_band".to_string(),
                IndicatorPlot::new("Lower Band".to_string(), lower, self.lower_band_color.clone()),
            );
        }

        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            current_time,
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
        self.is_ready = false;
        self.cumulative_pv = dec!(0.0);
        self.cumulative_volume = dec!(0.0);
        self.squared_diff_sum = dec!(0.0);
        self.last_reset_day = None;
        self.was_last_bar_in_session = false;
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
        1 // VWAP only needs one bar to start calculating
    }
}
