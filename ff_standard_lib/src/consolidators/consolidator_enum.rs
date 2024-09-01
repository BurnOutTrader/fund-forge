use chrono::{DateTime, Duration, Utc};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::consolidators::candlesticks::CandleStickConsolidator;
use crate::consolidators::count::{CountConsolidator};
use crate::consolidators::heikinashi::HeikinAshiConsolidator;
use crate::consolidators::renko::RenkoConsolidator;
use crate::standardized_types::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::subscriptions::{filter_resolutions, CandleType, DataSubscription};


pub enum ConsolidatorEnum {
    Count(CountConsolidator),
    CandleStickConsolidator(CandleStickConsolidator),
    HeikinAshi(HeikinAshiConsolidator),
    Renko(RenkoConsolidator),
}

impl ConsolidatorEnum {
    /// Creates a new consolidator based on the subscription. if is_warmed_up is true, the consolidator will warm up to the to_time on its own.
    pub async fn create_consolidator(is_warmed_up: bool, subscription: DataSubscription, history_to_retain: u64, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> ConsolidatorEnum {
        //todo handle errors here gracefully
        let is_tick = match subscription.resolution {
            Resolution::Ticks(_) => true,
            _ => false
        };

        if is_tick {
            return match is_warmed_up {
                true => {
                    let consolidator = ConsolidatorEnum::Count(CountConsolidator::new(subscription.clone(), history_to_retain).await.unwrap());
                    ConsolidatorEnum::warmup(consolidator, to_time, strategy_mode).await
                },
                false => ConsolidatorEnum::Count(CountConsolidator::new(subscription.clone(), history_to_retain).await.unwrap()),
            }
        }

        let consolidator = match &subscription.candle_type {
            Some(candle_type) => {
                match candle_type {
                    CandleType::Renko => ConsolidatorEnum::Renko(RenkoConsolidator::new(subscription.clone(), history_to_retain).await.unwrap()),
                    CandleType::HeikinAshi => ConsolidatorEnum::HeikinAshi(HeikinAshiConsolidator::new(subscription.clone(), history_to_retain).await.unwrap()),
                    CandleType::CandleStick => ConsolidatorEnum::CandleStickConsolidator(CandleStickConsolidator::new(subscription.clone(), history_to_retain).await.unwrap())
                }
            },
            _ => panic!("Candle type is required for CandleStickConsolidator")
        };
        
        match is_warmed_up {
            true => ConsolidatorEnum::warmup(consolidator, to_time, strategy_mode).await,
            false => consolidator
        }
    }

    /// Updates the consolidator with the new data point.
    pub fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => {
                count_consolidator.update(base_data)
            },
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => {
                time_consolidator.update(base_data)
            },
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => {
                heikin_ashi_consolidator.update(base_data)
            }
            ConsolidatorEnum::Renko(renko_consolidator) => {
                renko_consolidator.update(base_data)
            }
        }
    }

    /// Clears the current data and history of the consolidator.
    pub fn subscription(&self) -> &DataSubscription {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => &count_consolidator.subscription,
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => &time_consolidator.subscription,
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => &heikin_ashi_consolidator.subscription,
            ConsolidatorEnum::Renko(renko_consolidator) => &renko_consolidator.subscription,
        }
    }

    /// Returns the resolution of the consolidator.
    pub fn resolution(&self) -> &Resolution {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => &count_consolidator.subscription.resolution,
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => &time_consolidator.subscription.resolution,
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => &heikin_ashi_consolidator.subscription.resolution,
            ConsolidatorEnum::Renko(renko_consolidator) => &renko_consolidator.subscription.resolution,
        }
    }

    /// Returns the history to retain for the consolidator.
    pub fn history_to_retain(&self) -> u64 {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.history.number,
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => time_consolidator.history.number,
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.history.number,
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.history.number,
        }
    }

    pub fn history(&self) -> RollingWindow<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.history(),
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => time_consolidator.history(),
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.history(),
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.history(),
        }
    }

    pub fn current(&self) -> Option<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.current(),
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => time_consolidator.current(),
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.current(),
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.current(),
        }
    }

    pub fn index(&self, index: u64) -> Option<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.index(index),
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => time_consolidator.index(index),
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.index(index),
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.index(index),
        }
    }

    pub fn update_time(&mut self, time: DateTime<Utc>) -> Vec<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(_) => vec![],
            ConsolidatorEnum::CandleStickConsolidator(time_consolidator) => time_consolidator.update_time(time),
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.update_time(time),
            ConsolidatorEnum::Renko(_) => vec![],
        }
    }

    pub async fn warmup(mut consolidator: ConsolidatorEnum, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> ConsolidatorEnum {
        //todo if live we will tell the self.subscription.symbol.data_vendor to .update_historical_symbol()... we will wait then continue
        let subscription = consolidator.subscription();
        let vendor_resolutions = filter_resolutions(subscription.symbol.data_vendor.resolutions(subscription.market_type.clone()).await.unwrap(), consolidator.subscription().resolution);
        let max_resolution = vendor_resolutions.iter().max_by_key(|r| r.resolution);
        let min_resolution = match max_resolution.is_none() {
            true => panic!("{} does not have any resolutions available", subscription.symbol.data_vendor),
            false => max_resolution.unwrap()
        };

        let from_time = to_time - (subscription.resolution.as_duration() * consolidator.history().number as i32) - Duration::days(4); //we go back a bit further in case of holidays or weekends

        let base_subscription = DataSubscription::new(subscription.symbol.name.clone(), subscription.symbol.data_vendor.clone(), min_resolution.resolution, min_resolution.base_data_type, subscription.market_type.clone());
        let base_data = range_data(from_time, to_time, base_subscription.clone()).await;
        
        let mut last_time = from_time;
        let subscription_resolution_duration = Duration::seconds(1);
        while last_time < to_time {
            let time = last_time + subscription_resolution_duration;
            if let Some(slice) = base_data.get(&time) {
                for base_data in slice {
                    consolidator.update(base_data);
                }
            } else {
                consolidator.update_time(time);
            }
            last_time = time;
        }
        if strategy_mode != StrategyMode::Backtest {
            //todo() we will get any bars which are not in our serialized history here
        }
        consolidator
    }
}

