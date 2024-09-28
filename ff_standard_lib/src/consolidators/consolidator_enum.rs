use crate::consolidators::candlesticks::CandleStickConsolidator;
use crate::consolidators::count::CountConsolidator;
use crate::consolidators::heikinashi::HeikinAshiConsolidator;
use crate::consolidators::renko::RenkoConsolidator;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::subscriptions::{filter_resolutions, CandleType, DataSubscription};
use chrono::{DateTime, Duration, Utc};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::quotebar::QuoteBar;
use crate::standardized_types::time_slices::TimeSlice;

pub enum ConsolidatorEnum {
    Count(CountConsolidator),
    CandleStickConsolidator(CandleStickConsolidator),
    HeikinAshi(HeikinAshiConsolidator),
    Renko(RenkoConsolidator),
}

impl ConsolidatorEnum {
    /// Creates a new consolidator based on the subscription. if is_warmed_up is true, the consolidator will warm up to the to_time on its own.
    pub async fn create_consolidator(
        subscription: DataSubscription,
        fill_forward: bool
    ) -> ConsolidatorEnum {
        //todo handle errors here gracefully
        let is_tick = match subscription.resolution {
            Resolution::Ticks(_) => true,
            _ => false,
        };

        if is_tick {
           return ConsolidatorEnum::Count(
                CountConsolidator::new(subscription.clone())
                    .await
                    .unwrap(),
            )
        }

        let consolidator = match &subscription.candle_type {
            Some(candle_type) => match candle_type {
                CandleType::Renko => ConsolidatorEnum::Renko(
                    RenkoConsolidator::new(subscription.clone())
                        .await
                        .unwrap(),
                ),
                CandleType::HeikinAshi => ConsolidatorEnum::HeikinAshi(
                    HeikinAshiConsolidator::new(subscription.clone(), fill_forward)
                        .await
                        .unwrap(),
                ),
                CandleType::CandleStick => ConsolidatorEnum::CandleStickConsolidator(
                    CandleStickConsolidator::new(subscription.clone(), fill_forward)
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
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.update(base_data),
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
            ConsolidatorEnum::Renko(renko_consolidator) => &renko_consolidator.subscription,
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
            ConsolidatorEnum::Renko(renko_consolidator) => {
                &renko_consolidator.subscription.resolution
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
            ConsolidatorEnum::Renko(_) => None,
        }
    }

    pub async fn warmup(
        mut consolidator: ConsolidatorEnum,
        to_time: DateTime<Utc>,
        history_to_retain: i32,
        strategy_mode: StrategyMode,
    ) -> (ConsolidatorEnum, RollingWindow<BaseDataEnum>) {
        let subscription = consolidator.subscription();
        let vendor_resolutions = filter_resolutions(
            subscription
                .symbol
                .data_vendor
                .resolutions(subscription.market_type.clone())
                .await
                .unwrap(),
            consolidator.subscription().resolution,
        );
        let max_resolution = vendor_resolutions.iter().max_by_key(|r| r.resolution);
        let min_resolution = match max_resolution.is_none() {
            true => panic!(
                "{} does not have any resolutions available",
                subscription.symbol.data_vendor
            ),
            false => max_resolution.unwrap(),
        };

        let subtract_duration: Duration = consolidator.resolution().as_duration() * history_to_retain;
        let from_time = to_time - subtract_duration - Duration::days(5);

        let base_subscription = DataSubscription::new(
            subscription.symbol.name.clone(),
            subscription.symbol.data_vendor.clone(),
            min_resolution.resolution,
            min_resolution.base_data_type,
            subscription.market_type.clone(),
        );
        let primary_history = range_data(from_time, to_time, base_subscription.clone()).await;

        let mut last_time = from_time;
        let subscription_resolution_duration = consolidator.subscription().resolution.as_duration();
        let mut history = RollingWindow::new(history_to_retain as usize);
        while last_time < to_time {
            let time = last_time + subscription_resolution_duration;
            let time_slice: TimeSlice = primary_history
                .range(last_time..=time)
                .flat_map(|(_, value)| value.iter().cloned())
                .collect();
            if !time_slice.is_empty() {
                for base_data in time_slice {
                    let consolidated_data = consolidator.update(&base_data);
                    if let Some(closed_data) = consolidated_data.closed_data {
                        history.add(closed_data);
                    }
                }
            }
            let optional_consolidated_data = consolidator.update_time(time);
            if let Some(closed_data) = optional_consolidated_data {
                history.add(closed_data);
            }
            last_time = time;
        }
        if strategy_mode != StrategyMode::Backtest {
            //todo() we will get any bars which are not in our serialized history here
        }
        (consolidator, history)
    }

    pub async fn warmup_candles(
        mut consolidator: ConsolidatorEnum,
        to_time: DateTime<Utc>,
        history_to_retain: i32,
        strategy_mode: StrategyMode,
    ) -> (ConsolidatorEnum, RollingWindow<Candle>) {
        let subscription = consolidator.subscription();
        if subscription.base_data_type != BaseDataType::Candles {
            panic!("Incorrect base data type for fn warmup_candles()")
        }
        let vendor_resolutions = filter_resolutions(
            subscription
                .symbol
                .data_vendor
                .resolutions(subscription.market_type.clone())
                .await
                .unwrap(),
            consolidator.subscription().resolution,
        );
        let max_resolution = vendor_resolutions.iter().max_by_key(|r| r.resolution);
        let min_resolution = match max_resolution.is_none() {
            true => panic!(
                "{} does not have any resolutions available",
                subscription.symbol.data_vendor
            ),
            false => max_resolution.unwrap(),
        };

        let subtract_duration: Duration = consolidator.resolution().as_duration() * history_to_retain;
        let from_time = to_time - subtract_duration - Duration::days(5);

        let base_subscription = DataSubscription::new(
            subscription.symbol.name.clone(),
            subscription.symbol.data_vendor.clone(),
            min_resolution.resolution,
            min_resolution.base_data_type,
            subscription.market_type.clone(),
        );
        let primary_history = range_data(from_time, to_time, base_subscription.clone()).await;

        let mut last_time = from_time;
        let subscription_resolution_duration = consolidator.subscription().resolution.as_duration();
        let mut history = RollingWindow::new(history_to_retain as usize);
        while last_time < to_time {
            let time = last_time + subscription_resolution_duration;
            let time_slice: TimeSlice = primary_history
                .range(last_time..=time)
                .flat_map(|(_, value)| value.iter().cloned())
                .collect();
            if !time_slice.is_empty() {
                for base_data in time_slice {
                    let consolidated_data = consolidator.update(&base_data);
                    if let Some(closed_data) = consolidated_data.closed_data {
                        match closed_data {
                            BaseDataEnum::Candle(candle) =>  history.add(candle),
                            _ => {}
                        }
                    }
                }
            }
            if let Some(closed_data) =  consolidator.update_time(time) {
                match closed_data {
                    BaseDataEnum::Candle(candle) =>  history.add(candle),
                    _ => {}
                }
            }
            last_time = time;
        }
        if strategy_mode != StrategyMode::Backtest {
            //todo() we will get any bars which are not in our serialized history here
        }
        (consolidator, history)
    }

    pub async fn warmup_quotebars(
        mut consolidator: ConsolidatorEnum,
        to_time: DateTime<Utc>,
        history_to_retain: i32,
        strategy_mode: StrategyMode,
    ) -> (ConsolidatorEnum, RollingWindow<QuoteBar>) {
        let subscription = consolidator.subscription();
        if subscription.base_data_type != BaseDataType::QuoteBars {
            panic!("Incorrect base data type for fn warmup_candles()")
        }
        let vendor_resolutions = filter_resolutions(
            subscription
                .symbol
                .data_vendor
                .resolutions(subscription.market_type.clone())
                .await
                .unwrap(),
            consolidator.subscription().resolution,
        );
        let max_resolution = vendor_resolutions.iter().max_by_key(|r| r.resolution);
        let min_resolution = match max_resolution.is_none() {
            true => panic!(
                "{} does not have any resolutions available",
                subscription.symbol.data_vendor
            ),
            false => max_resolution.unwrap(),
        };

        let subtract_duration: Duration = consolidator.resolution().as_duration() * history_to_retain;
        let from_time = to_time - subtract_duration - Duration::days(5);

        let base_subscription = DataSubscription::new(
            subscription.symbol.name.clone(),
            subscription.symbol.data_vendor.clone(),
            min_resolution.resolution,
            min_resolution.base_data_type,
            subscription.market_type.clone(),
        );
        let primary_history = range_data(from_time, to_time, base_subscription.clone()).await;

        let mut last_time = from_time;
        let subscription_resolution_duration = consolidator.subscription().resolution.as_duration();
        let mut history = RollingWindow::new(history_to_retain as usize);
        while last_time < to_time {
            let time = last_time + subscription_resolution_duration;
            let time_slice: TimeSlice = primary_history
                .range(last_time..=time)
                .flat_map(|(_, value)| value.iter().cloned())
                .collect();
            if !time_slice.is_empty() {
                for base_data in time_slice {
                    let consolidated_data = consolidator.update(&base_data);
                    if let Some(closed_data) = consolidated_data.closed_data {
                        match closed_data {
                            BaseDataEnum::QuoteBar(qb) =>  history.add(qb),
                            _ => {}
                        }
                    }
                }
            }
            let optional_consolidated_data = consolidator.update_time(time);
            if let Some(closed_data) = optional_consolidated_data {
                match closed_data {
                    BaseDataEnum::QuoteBar(qb) =>  history.add(qb),
                    _ => {}
                }

            }
            last_time = time;
        }
        if strategy_mode != StrategyMode::Backtest {
            //todo() we will get any bars which are not in our serialized history here
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

