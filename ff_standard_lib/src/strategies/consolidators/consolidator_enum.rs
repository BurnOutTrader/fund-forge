use crate::strategies::consolidators::candlesticks::CandleStickConsolidator;
use crate::strategies::consolidators::count::CountConsolidator;
use crate::strategies::consolidators::heikinashi::HeikinAshiConsolidator;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::enums::{MarketType, StrategyMode};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::{filter_resolutions, CandleType, DataSubscription};
use chrono::{DateTime, Datelike, Duration, Utc, Weekday};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::history::get_historical_data;
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::futures_products::extract_symbol_from_contract;

pub enum ConsolidatorEnum {
    Count(CountConsolidator),
    CandleStickConsolidator(CandleStickConsolidator),
    HeikinAshi(HeikinAshiConsolidator),
}

impl ConsolidatorEnum {
    /// Creates a new consolidator based on the subscription. if is_warmed_up is true, the consolidator will warm up to the to_time on its own.
    pub async fn create_consolidator(
        subscription: DataSubscription,
        fill_forward: bool,
    ) -> ConsolidatorEnum {
        //todo handle errors here gracefully
        let is_tick = match subscription.resolution {
            Resolution::Ticks(_) => true,
            _ => false,
        };

        let symbol_name = match subscription.market_type {
            MarketType::Futures(_) => extract_symbol_from_contract(&subscription.symbol.name),
            _ => subscription.symbol.name.clone(),
        };

        let decimal_accuracy = subscription.symbol.data_vendor.decimal_accuracy(symbol_name.clone()).await.unwrap();
        let tick_size = subscription.symbol.data_vendor.tick_size(symbol_name.clone()).await.unwrap();

        if is_tick {
           return ConsolidatorEnum::Count(
                CountConsolidator::new(subscription.clone(), decimal_accuracy, tick_size)
                    .await
                    .unwrap(),
            )
        }

        let consolidator = match &subscription.candle_type {
            Some(candle_type) => match candle_type {
                CandleType::HeikinAshi => ConsolidatorEnum::HeikinAshi(
                    HeikinAshiConsolidator::new(subscription.clone(), fill_forward, decimal_accuracy, tick_size)
                        .await
                        .unwrap(),
                ),
                CandleType::CandleStick => ConsolidatorEnum::CandleStickConsolidator(
                    CandleStickConsolidator::new(subscription.clone(), fill_forward, decimal_accuracy, tick_size)
                        .await
                        .unwrap(),
                ),
            },
            _ => panic!("Candle type is required for CandleStickConsolidator"),
        };

       consolidator
    }

    /// Updates the consolidator with the new data point.
    pub fn update(&mut self, base_data: &BaseDataEnum) -> ConsolidatedData {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.update(base_data),
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => {
                time_consolidator.update(base_data)
            }
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => {
                heikin_ashi_consolidator.update(base_data)
            }
        }
    }

    /// Clears the current data and history of the consolidator.
    pub fn subscription(&self) -> &DataSubscription {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => &count_consolidator.subscription,
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => {
                &time_consolidator.subscription
            }
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => {
                &heikin_ashi_consolidator.subscription
            }
        }
    }

    /// Returns the resolution of the consolidator.
    pub fn resolution(&self) -> &Resolution {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => {
                &count_consolidator.subscription.resolution
            }
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => {
                &time_consolidator.subscription.resolution
            }
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => {
                &heikin_ashi_consolidator.subscription.resolution
            }
        }
    }

    /// Returns the history to retain for the consolidator.
    pub fn update_time(&mut self, time: DateTime<Utc>) -> Option<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(_) => None,
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => {
                time_consolidator.update_time(time)
            }
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => {
                heikin_ashi_consolidator.update_time(time)
            }
        }
    }

    pub async fn warmup(
        mut consolidator: ConsolidatorEnum,
        to_time: DateTime<Utc>,
        history_to_retain: i32,
        _strategy_mode: StrategyMode,
    ) -> (ConsolidatorEnum, RollingWindow<BaseDataEnum>) {
        let subscription = consolidator.subscription();
        let mut vendor_resolutions = filter_resolutions(
            subscription
                .symbol
                .data_vendor
                .resolutions(subscription.market_type.clone())
                .await
                .unwrap(),
            consolidator.subscription().resolution,
        );

        if subscription.candle_type == Some(CandleType::HeikinAshi) {
            vendor_resolutions.retain(|base_subscription| {
                base_subscription.base_data_type == BaseDataType::Ticks && base_subscription.resolution == Resolution::Ticks(1)
            });
        }
        let max_resolution = vendor_resolutions.iter().max_by_key(|r| r.resolution);
        let min_resolution = match max_resolution.is_none() {
            true => panic!(
                "{} does not have any resolutions available",
                subscription.symbol.data_vendor
            ),
            false => max_resolution.unwrap(),
        };

        let subtract_duration: Duration = consolidator.resolution().as_duration() * history_to_retain;
        let mut from_time = to_time - subtract_duration ;

        if to_time.weekday() == Weekday::Sun {
            from_time -= Duration::days(3);
        }

        let base_subscription = DataSubscription::new(
            subscription.symbol.name.clone(),
            subscription.symbol.data_vendor.clone(),
            min_resolution.resolution,
            min_resolution.base_data_type,
            subscription.market_type.clone(),
        );

        let mut history = RollingWindow::new(history_to_retain as usize);
        let data = match get_historical_data(vec![base_subscription.clone()], from_time, to_time).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("No data available or error: {}", e);
                return  (consolidator, history)
            }
        };

        for (_time, time_slice) in data {
            for base_data in time_slice.iter() {
                let consolidated_data = consolidator.update(&base_data);
                if let Some(closed_data) = consolidated_data.closed_data {
                    history.add(closed_data);
                }
            }
        }
        (consolidator, history)
    }
}

#[derive(Debug)]
pub struct ConsolidatedData {
    pub open_data: BaseDataEnum,
    pub closed_data: Option<BaseDataEnum>
}

impl ConsolidatedData {
    pub fn with_closed(open_data: BaseDataEnum, closed_data:BaseDataEnum) -> Self {
        Self {
            open_data,
            closed_data: Some(closed_data)
        }
    }

    pub fn with_open(open_data: BaseDataEnum) -> Self {
        Self {
            open_data,
            closed_data: None
        }
    }
}

