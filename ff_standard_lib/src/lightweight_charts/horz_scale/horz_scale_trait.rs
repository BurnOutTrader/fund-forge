use crate::standardized_types::TimeStamp;
use chrono::{DateTime, Offset, TimeZone};
use chrono_tz::Tz;

/// Represents a horizontal scale item, which is a data point without a value, where Time is an X axis value.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/WhitespaceData
pub trait HorzScaleItem {
    ///The utc time stamp of the data.
    fn timestamp_utc(&self) -> TimeStamp;

    ///Converts the Utc time stamp to local tz timestamp
    fn timestamp_local(&self, time_zone: &Tz) -> TimeStamp {
        let utc_time =
            DateTime::from_timestamp(self.timestamp_utc(), 0).expect("Invalid timestamp");
        let timezone_aware_datetime = time_zone.from_utc_datetime(&utc_time.naive_utc());
        let fixed_offset = time_zone
            .offset_from_utc_datetime(&utc_time.naive_utc())
            .fix();
        timezone_aware_datetime
            .with_timezone(&fixed_offset)
            .timestamp()
    }
}

/*/// Represents a whitespace data item, which is a data point without a value.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/WhitespaceData
pub trait WhiteSpaceData: HorzScaleItem {
    /// Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    fn custom_values(&self) -> Option<HashMap<String, String>>;
}*/
