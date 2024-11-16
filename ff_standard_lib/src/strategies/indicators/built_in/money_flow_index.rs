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

#[derive(Clone, Debug)]
pub struct MoneyFlowIndex {
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
    mfi_color: Color,
    overbought_color: Color,
    oversold_color: Color,
    divergence_color: Color,
    period: u64,
    tick_rounding: bool,
    overbought_level: Decimal,  // Typically 80
    oversold_level: Decimal,    // Typically 20
    last_typical_price: Option<Decimal>,
    raw_money_flows: Vec<(bool, Decimal)>, // (is_positive, money_flow)
}

impl Display for MoneyFlowIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl MoneyFlowIndex {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        period: u64,
        mfi_color: Color,
        overbought_color: Color,
        oversold_color: Color,
        divergence_color: Color,
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

        let mfi = MoneyFlowIndex {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(period as usize),
            is_ready: false,
            tick_size,
            mfi_color,
            overbought_color,
            oversold_color,
            divergence_color,
            period,
            decimal_accuracy,
            tick_rounding,
            overbought_level,
            oversold_level,
            last_typical_price: None,
            raw_money_flows: Vec::with_capacity(period as usize),
        };
        mfi
    }

    fn get_bar_data(data: &BaseDataEnum) -> Option<(Price, Price, Price, Volume)> {
        match data {
            BaseDataEnum::QuoteBar(bar) => Some((
                bar.bid_high,
                bar.bid_low,
                bar.bid_close,
                bar.bid_volume,
            )),
            BaseDataEnum::Candle(candle) => Some((
                candle.high,
                candle.low,
                candle.close,
                Decimal::from(candle.volume),
            )),
            _ => None,
        }
    }

    fn calculate_typical_price(high: Decimal, low: Decimal, close: Decimal) -> Decimal {
        (high + low + close) / dec!(3.0)
    }

    fn round_value(&self, value: Decimal) -> Price {
        match self.tick_rounding {
            true => round_to_tick_size(value, self.tick_size),
            false => value.round_dp(self.decimal_accuracy),
        }
    }

    fn calculate_money_flow(&mut self, high: Decimal, low: Decimal, close: Decimal, volume: Decimal) {
        let typical_price = Self::calculate_typical_price(high, low, close);
        let raw_money_flow = typical_price * volume;

        let is_positive = match self.last_typical_price {
            Some(last_price) => typical_price > last_price,
            None => true,
        };

        self.raw_money_flows.push((is_positive, raw_money_flow));
        if self.raw_money_flows.len() > self.period as usize {
            self.raw_money_flows.remove(0);
        }

        self.last_typical_price = Some(typical_price);
    }

    fn calculate_mfi(&self) -> Price {
        let (positive_flow, negative_flow): (Decimal, Decimal) = self.raw_money_flows.iter()
            .fold((dec!(0.0), dec!(0.0)), |acc, &(is_positive, flow)| {
                if is_positive {
                    (acc.0 + flow, acc.1)
                } else {
                    (acc.0, acc.1 + flow)
                }
            });

        if negative_flow == dec!(0.0) {
            return dec!(100.0);
        }

        let money_ratio = positive_flow / negative_flow;
        let mfi = dec!(100.0) - (dec!(100.0) / (dec!(1.0) + money_ratio));

        self.round_value(mfi)
    }

    fn detect_divergence(&self) -> Option<String> {
        if self.history.len() < 5 {
            return None;
        }

        let history: Vec<_> = self.history.history();
        let last_prices: Vec<_> = self.base_data_history.history()
            .iter()
            .filter_map(|data| Self::get_bar_data(data))
            .map(|(_, _, close, _)| close)
            .collect();

        let last_mfis: Vec<_> = history.iter()
            .filter_map(|v| v.plots.get("mfi"))
            .map(|p| p.value)
            .collect();

        if last_prices.len() < 2 || last_mfis.len() < 2 {
            return None;
        }

        // Check for bullish divergence (price making lower lows, MFI making higher lows)
        if last_prices[last_prices.len() - 1] < last_prices[last_prices.len() - 2] &&
            last_mfis[last_mfis.len() - 1] > last_mfis[last_mfis.len() - 2] {
            return Some("Bullish Divergence".to_string());
        }

        // Check for bearish divergence (price making higher highs, MFI making lower highs)
        if last_prices[last_prices.len() - 1] > last_prices[last_prices.len() - 2] &&
            last_mfis[last_mfis.len() - 1] < last_mfis[last_mfis.len() - 2] {
            return Some("Bearish Divergence".to_string());
        }

        None
    }

    fn get_signal(&self, mfi: Decimal) -> String {
        if mfi >= self.overbought_level {
            "Overbought".to_string()
        } else if mfi <= self.oversold_level {
            "Oversold".to_string()
        } else if mfi > dec!(50.0) {
            "Bullish".to_string()
        } else if mfi < dec!(50.0) {
            "Bearish".to_string()
        } else {
            "Neutral".to_string()
        }
    }
}

impl Indicators for MoneyFlowIndex {
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

        // Get price data
        let (high, low, close, volume) = Self::get_bar_data(base_data)?;

        // Update money flows
        self.calculate_money_flow(high, low, close, volume);

        if !self.is_ready {
            if self.raw_money_flows.len() < self.period as usize {
                return None;
            }
            self.is_ready = true;
        }

        // Calculate MFI
        let mfi = self.calculate_mfi();

        // Generate signal and check for divergence
        let signal = self.get_signal(mfi);
        let divergence = self.detect_divergence();

        // Create plots
        let mut plots = BTreeMap::new();

        // Main MFI line
        plots.insert(
            "mfi".to_string(),
            IndicatorPlot::new("MFI".to_string(), mfi, self.mfi_color.clone()),
        );

        // Reference levels
        plots.insert(
            "overbought".to_string(),
            IndicatorPlot::new("Overbought".to_string(), self.overbought_level, self.overbought_color.clone()),
        );
        plots.insert(
            "oversold".to_string(),
            IndicatorPlot::new("Oversold".to_string(), self.oversold_level, self.oversold_color.clone()),
        );

        // Signal
        plots.insert(
            "signal".to_string(),
            IndicatorPlot::new(signal, mfi, self.mfi_color.clone()),
        );

        // Divergence (if detected)
        if let Some(div) = divergence {
            plots.insert(
                "divergence".to_string(),
                IndicatorPlot::new(div, mfi, self.divergence_color.clone()),
            );
        }

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
        self.last_typical_price = None;
        self.raw_money_flows.clear();
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
        self.period + 1
    }
}
