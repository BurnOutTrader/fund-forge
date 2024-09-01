use crate::lightweight_charts::horz_scale::horz_scale_trait::HorzScaleItem;
use crate::standardized_types::{Color, Price, TimeStamp};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// In place of traits I am using EnumVariant(Struct) to allow easier 0 copy serde. \
/// A base interface for a data point of single-value series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/SingleValueData
pub enum SingleValueData {
    /// Structure describing a single item of data for area series.
    /// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/AreaData
    AreaData(AreaData),
    /// Structure describing a single item of data for baseline series.
    /// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/BaselineData
    BaselineData(BaselineData),
    /// Structure describing a single item of data for histogram series.
    /// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/HistogramData
    HistogramData(HistogramData),
    /// Structure describing a single item of data for line series.
    /// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/LineData
    LineData(LineData),
}

impl HorzScaleItem for SingleValueData {
    ///The utc time stamp of the data. use timestamp_local(Tz) to localise charts
    fn timestamp_utc(&self) -> TimeStamp {
        match self {
            SingleValueData::AreaData(area) => area.timestamp_utc(),
            SingleValueData::BaselineData(line) => line.timestamp_utc(),
            SingleValueData::HistogramData(hist) => hist.timestamp_utc(),
            SingleValueData::LineData(line) => line.timestamp_utc(),
        }
    }
}

impl SingleValueData {
    pub fn value(&self) -> Price {
        match self {
            SingleValueData::AreaData(area) => area.value,
            SingleValueData::BaselineData(line) => line.value,
            SingleValueData::HistogramData(hist) => hist.value,
            SingleValueData::LineData(line) => line.value,
        }
    }
}

/*impl WhiteSpaceData for SingleValueData {
    /// Represents a whitespace data item, which is a data point without a value.
    /// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/WhitespaceData
    /// Optional customValues: Record<string, unknown>
    /// Additional custom values which will be ignored by the library, but could be used by plugins.
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        match self {
            SingleValueData::AreaData(area) => area.custom_values(),
            SingleValueData::BaselineData(line) => line.custom_values(),
            SingleValueData::HistogramData(hist) => hist.custom_values(),
            SingleValueData::LineData(line) => line.custom_values(),
        }
    }
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Structure describing a single item of data for area series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/AreaData
pub struct AreaData {
    ///The utc time stamp of the data.
    pub time_utc: TimeStamp,

    /// Optional line color value for certain data item.
    /// If missed, color from options is used.
    line_color: Option<Color>,

    /// Optional top color value for certain data item.
    /// If missed, color from options is used.
    top_color: Option<Color>,

    /// Optional bottom color value for certain data item.
    /// If missed, color from options is used.
    bottom_color: Option<Color>,

    /// The price value of the data.
    value: Price,
    // Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    //pub custom_values: Option<HashMap<String, String>>
}

impl HorzScaleItem for AreaData {
    fn timestamp_utc(&self) -> TimeStamp {
        self.time_utc
    }
}
/*impl WhiteSpaceData for AreaData {
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        self.custom_values.clone()
    }
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Structure describing a single item of data for baseline series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/BaselineData
pub struct BaselineData {
    ///The utc time stamp of the data.
    pub time_utc: TimeStamp,

    /// Optional top area top fill color value for certain data item.
    /// If missed, color from options is used.
    pub top_fill_color1: Option<Color>,

    /// Optional top area bottom fill color value for certain data item.
    /// If missed, color from options is used.
    pub top_fill_color2: Option<Color>,

    /// Optional top area line color value for certain data item.
    /// If missed, color from options is used.
    pub top_line_color: Option<Color>,

    /// Optional bottom area top fill color value for certain data item.
    /// If missed, color from options is used.
    pub bottom_fill_color1: Option<Color>,

    /// Optional bottom area bottom fill color value for certain data item.
    /// If missed, color from options is used.
    pub bottom_fill_color2: Option<Color>,

    /// Optional bottom area line color value for certain data item.
    /// If missed, color from options is used.
    pub bottom_line_color: Option<Color>,

    /// The price value of the data.
    pub value: Price,
    // Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    //pub custom_values: Option<HashMap<String, String>>
}

impl HorzScaleItem for BaselineData {
    fn timestamp_utc(&self) -> TimeStamp {
        self.time_utc
    }
}
/*impl WhiteSpaceData for BaselineData {
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        self.custom_values.clone()
    }
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Structure describing a single item of data for histogram series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/HistogramData
pub struct HistogramData {
    ///The utc time stamp of the data.
    pub time_utc: TimeStamp,

    /// Optional color value for certain data item.
    /// If missed, color from options is used.
    pub color: Option<Color>,

    /// The price value of the data.
    pub value: Price,
    // Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    //pub custom_values: Option<HashMap<String, String>>
}

impl HorzScaleItem for HistogramData {
    fn timestamp_utc(&self) -> TimeStamp {
        self.time_utc
    }
}
/*impl WhiteSpaceData for HistogramData {
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        self.custom_values.clone()
    }
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Structure describing a single item of data for line series.
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/LineData
pub struct LineData {
    ///The utc time stamp of the data.
    pub time_utc: TimeStamp,

    /// Optional color value for certain data item.
    /// If missed, color from options is used.
    pub color: Option<Color>,

    /// The price value of the data.
    pub value: Price,
    // Optional customValues: Record<string, unknown>
    // Additional custom values which will be ignored by the library, but could be used by plugins.
    //pub custom_values: Option<HashMap<String, String>>
}

impl LineData {
    pub fn new(time_utc: TimeStamp, color: Option<Color>, value: Price) -> Self {
        Self {
            time_utc,
            color,
            value,
        }
    }
}

impl HorzScaleItem for LineData {
    fn timestamp_utc(&self) -> TimeStamp {
        self.time_utc
    }
}
/*impl WhiteSpaceData for LineData {
    fn custom_values(&self) -> Option<HashMap<String, String>> {
        self.custom_values.clone()
    }
}*/
