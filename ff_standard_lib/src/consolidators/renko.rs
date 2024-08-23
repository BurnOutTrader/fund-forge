use chrono::{DateTime, Duration, Utc};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::consolidators::count::ConsolidatorError;
use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::candle::Candle;
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::subscriptions::{CandleType, DataSubscription};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct RenkoParameters {
    
}

/// A consolidator that produces a new piece of data after a certain number of data points have been added.
/// Supports Ticks only.
pub struct RenkoConsolidator {
    current_data: Candle,
    pub(crate) subscription: DataSubscription,
    pub(crate) history: RollingWindow,
    parameters: RenkoParameters,
}

impl RenkoConsolidator {
    pub(crate) fn new(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError> {
        let current_data = match &subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Instant, subscription.candle_type.clone().unwrap()),
            _ => return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for CountConsolidator", subscription.base_data_type) }),
        };

        let parameters = match subscription.candle_type.clone() {
            Some(candle_type) => {
                match candle_type {
                    CandleType::Renko(params) => params,
                    _ => panic!("Unsupported candle type for RenkoConsolidator"),
                }
            }
            _ => panic!("RenkoConsolidator requires a candle type"),
        };

        Ok(RenkoConsolidator {
            current_data,
            subscription,
            history: RollingWindow::new(history_to_retain),
            parameters: parameters.clone(),
        })
    }

    pub(crate) async fn new_and_warmup(subscription: DataSubscription, history_to_retain: usize, warm_up_to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError> {
        let current_data = match subscription.base_data_type {
            BaseDataType::Ticks => Candle::new(subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Instant, subscription.candle_type.clone().unwrap()),
            _ => return Err(ConsolidatorError { message: format!("{} is an Invalid base data type for CountConsolidator", subscription.base_data_type) }),
        };
        
        let parameters = match subscription.candle_type.clone() {
            Some(candle_type) => {
                match candle_type {
                    CandleType::Renko(params) => params,
                    _ => panic!("Unsupported candle type for RenkoConsolidator"),
                }
            }
            _ => panic!("RenkoConsolidator requires a candle type"),
        };

        let mut consolidator = RenkoConsolidator {
            current_data,
            subscription,
            history: RollingWindow::new(history_to_retain),
            parameters: parameters.clone(),
        };
        
        consolidator.warmup(warm_up_to_time, strategy_mode).await;
        Ok(consolidator)
    }

    /// Returns a candle if the count is reached
    pub(crate) fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum> {
        todo!() //will need to be based on renko parameters
    }
    
    pub(crate) fn clear_current_data(&mut self) {
        self.current_data = Candle::new(self.subscription.symbol.clone(), 0.0, 0.0, "".to_string(), Resolution::Instant, self.subscription.candle_type.clone().unwrap());
        self.history.clear();
    }

    fn history(&self) -> &RollingWindow {
        &self.history
    }


    fn index(&self, index: usize) -> Option<BaseDataEnum> {
        match self.history.get(index) {
            Some(data) => Some(data.clone()),
            None => None,
        }
    }

    fn current(&self) -> Option<BaseDataEnum> {
        Some(BaseDataEnum::Candle(self.current_data.clone()))
    }

    async fn warmup(&mut self, to_time: DateTime<Utc>, strategy_mode: StrategyMode) {
        //todo if live we will tell the self.subscription.symbol.data_vendor to .update_historical_symbol()... we will wait then continue
        let vendor_resolutions = self.subscription.symbol.data_vendor.resolutions(self.subscription.market_type.clone()).await.unwrap();
        let mut minimum_resolution: Option<Resolution> = None;
        for resolution in vendor_resolutions {
            if minimum_resolution.is_none() {
                minimum_resolution = Some(resolution);
            } else {
                if resolution > minimum_resolution.unwrap() && resolution < self.subscription.resolution {
                    minimum_resolution = Some(resolution);
                }
            }
        }

        let minimum_resolution = match minimum_resolution.is_none() {
            true => panic!("{} does not have any resolutions available", self.subscription.symbol.data_vendor),
            false => minimum_resolution.unwrap()
        };

        let data_type = match minimum_resolution {
            Resolution::Ticks(_) => BaseDataType::Ticks,
            _ => self.subscription.base_data_type.clone()
        };

        let from_time = to_time - (self.subscription.resolution.as_duration() * self.history.number as i32) - Duration::days(4); //we go back a bit further in case of holidays or weekends

        let base_subscription = DataSubscription::new(self.subscription.symbol.name.clone(), self.subscription.symbol.data_vendor.clone(), minimum_resolution, data_type, self.subscription.market_type.clone());
        let base_data = range_data(from_time, to_time, base_subscription.clone()).await;

        for (_, slice) in &base_data {
            for base_data in slice {
                self.update(base_data);
            }
        }
        if strategy_mode != StrategyMode::Backtest {
            //todo() we will get any bars which are not in out serialized history here
        }
    }
}