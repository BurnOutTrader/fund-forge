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
pub struct KeltnerChannels {
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
    middle_color: Color,
    upper_color: Color,
    lower_color: Color,
    ema_period: u64,
    atr_period: u64,
    multiplier: Decimal,
    tick_rounding: bool,
    last_ema: Option<Decimal>,
    last_atr: Option<Decimal>,
}

impl Display for KeltnerChannels {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let last = self.history.last();
        match last {
            Some(last) => write!(f, "{}\n{}", &self.name, last),
            None => write!(f, "{}: No Values", &self.name),
        }
    }
}

impl KeltnerChannels {
    #[allow(dead_code)]
    pub async fn new(
        name: IndicatorName,
        subscription: DataSubscription,
        history_to_retain: usize,
        ema_period: u64,
        atr_period: u64,
        multiplier: Decimal,
        middle_color: Color,
        upper_color: Color,
        lower_color: Color,
        tick_rounding: bool,
    ) -> Self {
        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };
        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        let keltner = KeltnerChannels {
            name,
            market_type: subscription.symbol.market_type.clone(),
            subscription,
            history: RollingWindow::new(history_to_retain),
            base_data_history: RollingWindow::new(atr_period.max(ema_period) as usize + 1),
            is_ready: false,
            tick_size,
            middle_color,
            upper_color,
            lower_color,
            ema_period,
            atr_period,
            multiplier,
            decimal_accuracy,
            tick_rounding,
            last_ema: None,
            last_atr: None,
        };
        keltner
    }

    fn get_price_data(data: &BaseDataEnum) -> Option<(Price, Price, Price)> {
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

    fn calculate_true_range(&self, current: &BaseDataEnum, previous: Option<&BaseDataEnum>) -> Price {
        let (current_high, current_low, current_close) = Self::get_price_data(current).unwrap();

        let prev_close = previous
            .and_then(Self::get_price_data)
            .map(|(_, _, close)| close)
            .unwrap_or(current_close);

        let high_low = current_high - current_low;
        let high_close = (current_high - prev_close).abs();
        let low_close = (current_low - prev_close).abs();

        high_low.max(high_close).max(low_close)
    }

    fn calculate_atr(&self, true_range: Decimal) -> Price {
        match self.last_atr {
            None => true_range,
            Some(last_atr) => {
                let period_dec = Decimal::from(self.atr_period);
                (last_atr * (period_dec - dec!(1.0)) + true_range) / period_dec
            }
        }
    }

    fn calculate_ema(&self, current_price: Decimal) -> Price {
        let ema_multiplier = dec!(2.0) / (Decimal::from(self.ema_period) + dec!(1.0));

        match self.last_ema {
            None => current_price,
            Some(last_ema) => {
                current_price * ema_multiplier + last_ema * (dec!(1.0) - ema_multiplier)
            }
        }
    }

    fn round_value(&self, value: Decimal) -> Price {
        match self.tick_rounding {
            true => round_to_tick_size(value, self.tick_size),
            false => value.round_dp(self.decimal_accuracy),
        }
    }
}

impl Indicators for KeltnerChannels {
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
        let (_, _, close) = Self::get_price_data(base_data)?;

        // Calculate EMA (middle line)
        let ema = self.calculate_ema(close);
        self.last_ema = Some(ema);

        // Calculate ATR for the bands
        let true_range = self.calculate_true_range(
            base_data,
            history.get(history.len() - 2),
        );

        let atr = self.calculate_atr(true_range);
        self.last_atr = Some(atr);

        // Calculate bands
        let band_offset = atr * self.multiplier;
        let upper = self.round_value(ema + band_offset);
        let middle = self.round_value(ema);
        let lower = self.round_value(ema - band_offset);

        // Create plots
        let mut plots = BTreeMap::new();
        plots.insert(
            "middle".to_string(),
            IndicatorPlot::new("Middle".to_string(), middle, self.middle_color.clone()),
        );
        plots.insert(
            "upper".to_string(),
            IndicatorPlot::new("Upper".to_string(), upper, self.upper_color.clone()),
        );
        plots.insert(
            "lower".to_string(),
            IndicatorPlot::new("Lower".to_string(), lower, self.lower_color.clone()),
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
        self.last_ema = None;
        self.last_atr = None;
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
        self.ema_period.max(self.atr_period) + 1
    }
}
