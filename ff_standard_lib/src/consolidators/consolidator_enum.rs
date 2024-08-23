use chrono::{DateTime, Utc};
use crate::consolidators::candlesticks::CandleStickConsolidator;
use crate::consolidators::consolidators_trait::Consolidators;
use crate::consolidators::count::{CountConsolidator};
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
    
    pub async fn create_consolidator(is_warmed_up: bool, subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> ConsolidatorEnum {
        let is_tick = match subscription.resolution {
            Resolution::Ticks(_) => true,
            _ => false
        };
        
        if  is_tick {
            return match is_warmed_up {
                true => ConsolidatorEnum::Count(CountConsolidator::new_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap()),
                false => ConsolidatorEnum::Count(CountConsolidator::new(subscription.clone(), history_to_retain).unwrap()),
            }
        }
        
        if is_warmed_up {
            return match &subscription.candle_type {
                Some(candle_type) => {
                    match candle_type {
                        CandleType::Renko(_) => ConsolidatorEnum::Renko(RenkoConsolidator::new_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap()),
                        CandleType::HeikinAshi => ConsolidatorEnum::HeikinAshi(HeikinAshiConsolidator::new_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap()),
                        CandleType::CandleStick => ConsolidatorEnum::TimeCandlesOrQuoteBars(CandleStickConsolidator::new_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap())
                    }
                },
                None => ConsolidatorEnum::TimeCandlesOrQuoteBars(CandleStickConsolidator::new_and_warmup(subscription.clone(), history_to_retain, to_time, strategy_mode).await.unwrap())
            }
        }
        
        match &subscription.candle_type {
            Some(candle_type) => {
                match candle_type {
                    CandleType::Renko(_) => ConsolidatorEnum::Renko(RenkoConsolidator::new(subscription.clone(), history_to_retain).unwrap()),
                    CandleType::HeikinAshi => ConsolidatorEnum::HeikinAshi(HeikinAshiConsolidator::new(subscription.clone(), history_to_retain).unwrap()),
                    CandleType::CandleStick => ConsolidatorEnum::TimeCandlesOrQuoteBars(CandleStickConsolidator::new(subscription.clone(), history_to_retain).unwrap())
                }
            },
            None => panic!("Candle type is required for this subscription")
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