use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::lightweight_charts::horz_scale::horz_scale_trait::HorzScaleItem;
use crate::standardized_types::{Color, Price, TimeStamp};

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Represents a bar with a Time and open, high, low, and close prices.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/OhlcData
pub enum OhlcData  {
    CandleStickData(CandleStickData),
    BarData(BarData)
}

impl OhlcData {

    /// The open price
    ///https://tradingview.github.io/lightweight-charts/docs/api/interfaces/OhlcData
    pub fn open(&self) -> Price {
        match self  {
            OhlcData::CandleStickData(candle) => candle.open,
            OhlcData::BarData(bar) => bar.open
        }
    }

    /// The close price
    ///https://tradingview.github.io/lightweight-charts/docs/api/interfaces/OhlcData
    pub fn close(&self) -> Price {
        match self  {
            OhlcData::CandleStickData(candle) => candle.close,
            OhlcData::BarData(bar) => bar.close
        }
    }

    /// The high price
    ///https://tradingview.github.io/lightweight-charts/docs/api/interfaces/OhlcData
    pub fn high(&self) -> Price {
        match self  {
            OhlcData::CandleStickData(candle) => candle.high,
            OhlcData::BarData(bar) => bar.high
        }
    }

    /// The low price
    ///https://tradingview.github.io/lightweight-charts/docs/api/interfaces/OhlcData
    pub fn low(&self) -> Price {
        match self  {
            OhlcData::CandleStickData(candle) => candle.low,
            OhlcData::BarData(bar) => bar.low
        }
    }
}
impl HorzScaleItem for OhlcData {

    ///The utc time stamp of the data. use timestamp_local(Tz) to localise charts
    fn timestamp_utc(&self) -> TimeStamp {
        match self {
            OhlcData::CandleStickData(candles) => candles.timestamp_utc(),
            OhlcData::BarData(bar) => bar.timestamp_utc()
        }
    }
}
/*impl WhiteSpaceData for OhlcData {

    /// Represents a whitespace data item, which is a data point without a value.
    /// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/WhitespaceData
    /// Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        match self {
            OhlcData::CandleStickData(candles) => candles.custom_values(),
            OhlcData::BarData(bar) => bar.custom_values()
        }
    }
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Structure describing a single item of data for candlestick series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/CandlestickData
pub struct CandleStickData {
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

    /// Optional border color value for certain data item. If missed, color from options is used.
    pub border_color: Option<Color>,

    /// Optional wick color value for certain data item. If missed, color from options is used.
    pub wick_color: Option<Color>,

    // Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    //pub custom_values: Option<HashMap<String, String>>
}

impl CandleStickData {
    pub fn new(time_utc: TimeStamp, open: Price, high: Price, low: Price, close: Price, color: Option<Color>, border_color: Option<Color>, wick_color: Option<Color>) -> Self {
        CandleStickData {
            time_utc,
            open,
            high,
            low,
            close,
            color,
            border_color,
            wick_color
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
#[archive(
    compare(PartialEq),
    check_bytes,
)]
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
    pub  fn new(time_utc: TimeStamp, open: Price, high: Price, low: Price,
                close: Price, color: Option<Color>) -> Self {
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
impl HorzScaleItem for BarData {
    fn timestamp_utc(&self) -> TimeStamp {
        self.time_utc
    }
}
/*impl WhiteSpaceData for BarData {
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        self.custom_values.clone()
    }
}*/

