use chrono::{DateTime, TimeZone, Utc};
use crate::rolling_window::RollingWindow;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::subscriptions::DataSubscription;

#[derive(Debug, Clone)]
pub struct IndicatorResult {
    value: f64,
    time: DateTime<Utc>,
    plot_name: String,
}

impl IndicatorResult {
    pub fn new(value: f64, time: DateTime<Utc>, plot_name: String) -> IndicatorResult {
        IndicatorResult {
            value,
            time,
            plot_name
        }
    }
    
    pub fn timestamp(&self) -> i64 {
        self.time.timestamp()
    }
    
    pub fn value(&self) -> f64 {
        self.value
    }
    
    pub fn plot_name(&self) -> &str {
        &self.plot_name
    }
}

pub trait Indicator {
    fn name(&self) -> &str;
    fn subscription(&self) -> &DataSubscription;
    fn update(&mut self, base_data: BaseDataEnum) -> Option<IndicatorResult>;
    fn reset(&mut self);
    fn index(&self, index: usize) -> Option<IndicatorResult>;
    
    /// used to return the full history of the indicator for charting purposes
    fn plots(&self) -> RollingWindow<IndicatorResult>;
}