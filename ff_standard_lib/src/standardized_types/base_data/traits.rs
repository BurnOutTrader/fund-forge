use crate::standardized_types::enums::MarketType;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use crate::standardized_types::datavendor_enum::DataVendor;
use crate::standardized_types::resolution::Resolution;

/// Properties are used to update the data que during strategy execution.
pub trait BaseData {
    fn symbol_name(&self) -> Symbol;

    /// The time of the data point in the specified FixedOffset time zone.
    fn time_local(&self, time_zone: &Tz) -> DateTime<Tz>;

    /// UTC time of the data point.
    fn time_utc(&self) -> DateTime<Utc>;
    fn time_closed_utc(&self) -> DateTime<Utc>;
    fn time_closed_local(&self, time_zone: &Tz) -> DateTime<Tz>;

    fn data_vendor(&self) -> DataVendor;

    fn market_type(&self) -> MarketType;

    fn resolution(&self) -> Resolution;

    fn symbol(&self) -> &Symbol;

    fn subscription(&self) -> DataSubscription;
}
