use crate::lightweight_charts::horz_scale::horz_scale_trait::HorzScaleItem;
use crate::standardized_types::{Color, Price, TimeStamp};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};


#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Structure describing a single item of data for candlestick series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/CandlestickData


impl CandleStickData {
    pub fn new(
        time: TimeStamp,
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        color: Option<Color>,
        border_color: Option<Color>,
        wick_color: Option<Color>,
    ) -> Self {
        CandleStickData {
            time_utc,
            open,
            high,
            low,
            close,
            color,
            border_color,
            wick_color,
        }
    }
}
impl HorzScaleItem for CandleStickData {
    fn timestamp_utc(&self) -> TimeStamp {
        self.time_utc
    }
}
/*impl WhiteSpaceData for CandleStickData {
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        self.custom_values.clone()
    }
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Structure describing a single item of data for bar series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/BarData
pub struct BarData {
    ///The utc time stamp of the data.
    pub time_utc: TimeStamp,

    /// The open price.
    pub open: Price,

    /// The high price.
    pub high: Price,

    /// The lowest price.
    pub low: Price,

    /// The close price.
    pub close: Price,

    /// Optional color value for certain data item. If missed, color from options is used.
    pub color: Option<Color>,
    // Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    //pub custom_values: Option<HashMap<String, String>>
}
impl BarData {
    pub fn new(
        time_utc: TimeStamp,
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        color: Option<Color>,
    ) -> Self {
        Self {
            time_utc,
            open,
            high,
            low,
            close,
            color,
            //custom_values,
        }
    }
}

impl BarData {
    pub fn timestamp_utc(&self) -> TimeStamp {
        self.time_utc
    }
}
/*impl WhiteSpaceData for BarData {
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        self.custom_values.clone()
    }
}*/
