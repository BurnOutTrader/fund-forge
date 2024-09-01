use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::lightweight_charts::horz_scale::horz_scale_trait::HorzScaleItem;
use crate::lightweight_charts::horz_scale::ohlc_enum::OhlcData;
use crate::lightweight_charts::horz_scale::single_value_data_enum::SingleValueData;
use crate::standardized_types::TimeStamp;

pub mod horz_scale_trait;
pub mod single_value_data_enum;
pub mod ohlc_enum;


#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Represents a whitespace data item, which is a data point without a value but with a timestamp
/// https://tradingview.github.io/lightweight-charts/docs/api/interfaces/WhitespaceData
pub enum HorizontalScaleItem {
    OhlcData(OhlcData),
    SingleValueData(SingleValueData)
}

impl HorzScaleItem for HorizontalScaleItem {
    fn timestamp_utc(&self) -> TimeStamp {
        match self {
            HorizontalScaleItem::OhlcData(ohlc) => ohlc.timestamp_utc(),
            HorizontalScaleItem::SingleValueData(svd) => svd.timestamp_utc()
        }
    }
}