use chrono::{DateTime, Utc};
use crate::consolidators::candlesticks::CandleStickConsolidator;
use crate::consolidators::count::{ConsolidatorError, CountConsolidator};
use crate::consolidators::heikinashi::HeikinAshiConsolidator;
use crate::consolidators::renko::RenkoConsolidator;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};

pub enum ConsolidatorEnum {
    Count(CountConsolidator),
    TimeCandlesOrQuoteBars(CandleStickConsolidator),
    HeikinAshi(HeikinAshiConsolidator),
    Renko(RenkoConsolidator)
}

impl ConsolidatorEnum {
    
    pub async fn create_consolidator(is_warmed_up: bool, is_tick: bool, subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> ConsolidatorEnum {
        match is_tick {
            false => {
                match is_warmed_up {
                    true => {
                        match &subscription.candle_type {
                            Some(candle_type) => {
                                match candle_type {
                                    CandleType::Renko(_) => ConsolidatorEnum::new_renko_consolidator_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap(),
                                    CandleType::HeikinAshi => ConsolidatorEnum::new_heikin_ashi_consolidator_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap(),
                                    CandleType::CandleStick => ConsolidatorEnum::new_time_consolidator_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap(),
                                }
                            },
                            None => ConsolidatorEnum::new_time_consolidator_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap()
                        }
                    },
                    false => {
                        match &subscription.candle_type {
                            Some(candle_type) => {
                                match candle_type {
                                    CandleType::Renko(_) => ConsolidatorEnum::new_renko_consolidator(subscription.clone(), history_to_retain).unwrap(),
                                    CandleType::HeikinAshi => ConsolidatorEnum::new_heikin_ashi_consolidator(subscription.clone(), history_to_retain).unwrap(),
                                    CandleType::CandleStick => ConsolidatorEnum::new_time_consolidator(subscription.clone(), history_to_retain).unwrap(),
                                }
                            },
                            None => ConsolidatorEnum::new_time_consolidator(subscription.clone(), history_to_retain).unwrap()
                        }
                    },
                }
            }
            true => {
                match is_warmed_up {
                    true => ConsolidatorEnum::new_count_consolidator_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap(),
                    false => ConsolidatorEnum::new_count_consolidator(subscription.clone(), history_to_retain).unwrap(),
                }
            }
        }
    }
    
    pub async fn new_count_consolidator_and_warmup(subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        match CountConsolidator::new_and_warmup(subscription, history_to_retain, to_time, strategy_mode).await {
            Ok(consolidator) => Ok(ConsolidatorEnum::Count(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }
    
    pub async fn new_renko_consolidator_and_warmup(subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        match RenkoConsolidator::new_and_warmup(subscription, history_to_retain, to_time, strategy_mode).await {
            Ok(consolidator) => Ok(ConsolidatorEnum::Renko(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }
    
    pub async fn new_heikin_ashi_consolidator_and_warmup(subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        match HeikinAshiConsolidator::new_and_warmup(subscription, history_to_retain, to_time, strategy_mode).await {
            Ok(consolidator) => Ok(ConsolidatorEnum::HeikinAshi(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub async fn new_time_consolidator_and_warmup(subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        match CandleStickConsolidator::new_and_warmup(subscription, history_to_retain, to_time, strategy_mode).await {
            Ok(consolidator) => Ok(ConsolidatorEnum::TimeCandlesOrQuoteBars(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }
    
    pub fn new_heikin_ashi_consolidator(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match HeikinAshiConsolidator::new(subscription, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::HeikinAshi(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn new_count_consolidator(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match CountConsolidator::new(subscription, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::Count(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn new_time_consolidator(subscription: DataSubscription,  history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match CandleStickConsolidator::new(subscription, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::TimeCandlesOrQuoteBars(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }
    
    pub fn new_renko_consolidator(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        match RenkoConsolidator::new(subscription, history_to_retain) {
            Ok(consolidator) => Ok(ConsolidatorEnum::Renko(consolidator)),
            Err(e) => Err(ConsolidatorError { message: e.message }),
        }
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => {
                count_consolidator.update(base_data)
            },
            ConsolidatorEnum::TimeCandlesOrQuoteBars(time_consolidator) => {
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

    pub fn subscription(&self) -> DataSubscription {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.subscription.clone(),
            ConsolidatorEnum::TimeCandlesOrQuoteBars(time_consolidator) => time_consolidator.subscription.clone(),
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.subscription.clone(),
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.subscription.clone(),
        }
    }
    
    pub fn resolution(&self) -> Resolution {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.subscription.resolution.clone(),
            ConsolidatorEnum::TimeCandlesOrQuoteBars(time_consolidator) => time_consolidator.subscription.resolution.clone(),
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.subscription.resolution.clone(),
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.subscription.resolution.clone(),
        }
    }
    
    pub fn history_to_retain(&self) -> usize {
        match self {
            ConsolidatorEnum::Count(count_consolidator) => count_consolidator.history.number,
            ConsolidatorEnum::TimeCandlesOrQuoteBars(time_consolidator) => time_consolidator.history.number,
            ConsolidatorEnum::HeikinAshi(heikin_ashi_consolidator) => heikin_ashi_consolidator.history.number,
            ConsolidatorEnum::Renko(renko_consolidator) => renko_consolidator.history.number,
        }
    }
}