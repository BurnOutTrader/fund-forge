use chrono::{DateTime, Duration, Utc};
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::consolidators::count::{ConsolidatorError};
use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::history::range_data;
use crate::standardized_types::enums::{Resolution, StrategyMode};
use crate::standardized_types::subscriptions::{DataSubscription};
use std::result::Result;

/// A trait for consolidators which produce a new piece of data from vendor base data subscriptions.
pub trait Consolidators {
    /// Create a new consolidator
    fn new(subscription: DataSubscription, history_to_retain: usize) -> Result<Self, ConsolidatorError>
    where
        Self: Sized;

    /// Create a new consolidator and warm it up with historical data
    async fn new_and_warmup(subscription: DataSubscription, history_to_retain: usize, to_time: DateTime<Utc>, strategy_mode: StrategyMode) -> Result<Self, ConsolidatorError>
    where
        Self: Sized + Consolidators;

    /// The subscription used to get data for the consolidator
    fn subscription(&self) -> DataSubscription;

    /// The resolution of the data the consolidator will produce
    fn resolution(&self) -> Resolution;

    /// The number of data points the consolidator will retain in its history
    fn history_to_retain(&self) -> usize;

    /// Update the consolidator with new data
    fn update(&mut self, base_data: &BaseDataEnum) -> Vec<BaseDataEnum>;

    /// Clear the current data from history
    fn clear_current_data(&mut self);

    /// Get the current data from the consolidator
    fn history(&self) -> &RollingWindow;

    fn index(&self, index: usize) -> Option<BaseDataEnum>;

    fn current(&self) -> Option<BaseDataEnum>;

    /// Warm up the consolidator with historical data
    async fn warmup(&mut self, to_time: DateTime<Utc>, strategy_mode: StrategyMode) {
        let subscription = self.subscription();
        //todo if live we will tell the self.subscription.symbol.data_vendor to .update_historical_symbol()... we will wait then continue
        let vendor_resolutions = subscription.symbol.data_vendor.resolutions(subscription.market_type.clone()).await.unwrap();
        let mut minimum_resolution: Option<Resolution> = None;
        for resolution in vendor_resolutions {
            if minimum_resolution.is_none() {
                minimum_resolution = Some(resolution);
            } else {
                if resolution > minimum_resolution.unwrap() && resolution < subscription.resolution {
                    minimum_resolution = Some(resolution);
                }
            }
        }

        let minimum_resolution = match minimum_resolution.is_none() {
            true => panic!("{} does not have any resolutions available", subscription.symbol.data_vendor),
            false => minimum_resolution.unwrap()
        };

        let data_type = match minimum_resolution {
            Resolution::Ticks(_) => BaseDataType::Ticks,
            _ => subscription.base_data_type.clone()
        };

        let from_time = to_time - (subscription.resolution.as_duration() * self.history().number as i32) - Duration::days(4); //we go back a bit further in case of holidays or weekends

        let base_subscription = DataSubscription::new(self.subscription().symbol.name.clone(), subscription.symbol.data_vendor.clone(), minimum_resolution, data_type, subscription.market_type.clone());
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