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
pub struct RelativeStrengthIndex {
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
    plot_color: Color,
    period: u64,
    tick_rounding: bool,
    last_avg_gain: Option<Decimal>,
    last_avg_loss: Option<Decimal>,
}

impl Display for RelativeStrengthIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl RelativeStrengthIndex {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        plot_color: Color,
        tick_rounding: bool,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let rsi = RelativeStrengthIndex {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new((period + 1) as usize), // Need +1 to calculate first change
            is_ready: false,
            tick_size,
            plot_color,
            period,
            decimal_accuracy,
            tick_rounding,
            last_avg_gain: None,
            last_avg_loss: None,
        };
        rsi
    }

    fn get_close_price(data: &BaseDataEnum) -> Price {
        match data {
            BaseDataEnum::QuoteBar(bar) => bar.bid_close,
            BaseDataEnum::Candle(candle) => candle.close,
            _ => panic!("Unsupported data type for RSI"),
        }
    }

    fn calculate_initial_averages(&self) -> (Decimal, Decimal) {
        let base_data = self.base_data_history.history();
        let mut gains = Vec::new();
        let mut losses = Vec::new();

        // Calculate price changes
        for i in 1..base_data.len() {
            let prev_price = Self::get_close_price(&base_data[i - 1]);
            let curr_price = Self::get_close_price(&base_data[i]);
            let change = curr_price - prev_price;

            if change > dec!(0.0) {
                gains.push(change);
                losses.push(dec!(0.0));
            } else {
                gains.push(dec!(0.0));
                losses.push(change.abs());
            }
        }

        // Calculate averages
        let avg_gain = gains.iter().sum::<Decimal>() / Decimal::from(self.period);
        let avg_loss = losses.iter().sum::<Decimal>() / Decimal::from(self.period);

        match self.tick_rounding {
            true => (
                round_to_tick_size(avg_gain, self.tick_size),
                round_to_tick_size(avg_loss, self.tick_size),
            ),
            false => (
                avg_gain.round_dp(self.decimal_accuracy),
                avg_loss.round_dp(self.decimal_accuracy),
            ),
        }
    }

    fn calculate_rsi(&self, avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
        if avg_loss == dec!(0.0) {
            return dec!(100.0);
        }

        let rs = avg_gain / avg_loss;
        let rsi = dec!(100.0) - (dec!(100.0) / (dec!(1.0) + rs));

        match self.tick_rounding {
            true => round_to_tick_size(rsi, self.tick_size),
            false => rsi.round_dp(self.decimal_accuracy),
        }
    }

    fn calculate_smoothed_averages(
        &self,
        last_avg_gain: Decimal,
        last_avg_loss: Decimal,
        current_gain: Decimal,
        current_loss: Decimal,
    ) -> (Decimal, Decimal) {
        // Use Wilder's smoothing: [(prev_avg * (period - 1)) + current_value] / period
        let period_dec = Decimal::from(self.period);
        let multiplier = Decimal::from(self.period - 1);

        let avg_gain = (last_avg_gain * multiplier + current_gain) / period_dec;
        let avg_loss = (last_avg_loss * multiplier + current_loss) / period_dec;

        match self.tick_rounding {
            true => (
                round_to_tick_size(avg_gain, self.tick_size),
                round_to_tick_size(avg_loss, self.tick_size),
            ),
            false => (
                avg_gain.round_dp(self.decimal_accuracy),
                avg_loss.round_dp(self.decimal_accuracy),
            ),
        }
    }
}

impl Indicators for RelativeStrengthIndex {
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

            // Initialize with first set of averages
            let (avg_gain, avg_loss) = self.calculate_initial_averages();
            self.last_avg_gain = Some(avg_gain);
            self.last_avg_loss = Some(avg_loss);
            self.is_ready = true;
        }

        // Get current and previous prices
        let base_data = self.base_data_history.history();
        let curr_price = Self::get_close_price(&base_data[base_data.len() - 1]);
        let prev_price = Self::get_close_price(&base_data[base_data.len() - 2]);

        // Calculate current change
        let change = curr_price - prev_price;
        let (current_gain, current_loss) = if change > dec!(0.0) {
            (change, dec!(0.0))
        } else {
            (dec!(0.0), change.abs())
        };

        // Update averages using Wilder's smoothing
        let (new_avg_gain, new_avg_loss) = self.calculate_smoothed_averages(
            self.last_avg_gain.unwrap(),
            self.last_avg_loss.unwrap(),
            current_gain,
            current_loss,
        );

        // Calculate RSI
        let rsi = self.calculate_rsi(new_avg_gain, new_avg_loss);

        // Update state for next calculation
        self.last_avg_gain = Some(new_avg_gain);
        self.last_avg_loss = Some(new_avg_loss);

        // Create indicator values
        let mut plots = BTreeMap::new();
        plots.insert(
            "rsi".to_string(),
            IndicatorPlot::new("rsi".to_string(), rsi, self.plot_color.clone()),
        );

        let values = IndicatorValues::new(
            self.name.clone(),
            self.subscription.clone(),
            plots,
            base_data.last().unwrap().time_closed_utc(),
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
        self.last_avg_gain = None;
        self.last_avg_loss = None;
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
        self.history.len() as u64 + self.period + 1 // +1 for initial change calculation
    }
}

