use std::str::FromStr;
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use chrono_tz::Tz::UTC;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use ff_standard_lib::app::settings::ColorTheme;
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::traits::bytes::Bytes;
use crate::drawing_tool_enum::DrawingTool;

/// Used by strategies to communicate with the application remotely about new chart configurations. Uses rkyv serialization and the ff Bytes Trait
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct StrategyChartFlags {
    owner: OwnerId,
    subscription: DataSubscription,
    drawing_objects: Vec<u8>,
    time_zone: String,
    from_date: String,
    to_date: String,
    theme: ColorTheme
}

impl Bytes<Self> for StrategyChartFlags {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 2048>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<StrategyChartFlags, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<StrategyChartFlags>(archived) { //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

impl StrategyChartFlags {

    /// Deserializes the drawing tools back from bytes
    pub fn drawing_objects(&self) -> Vec<DrawingTool> {
        DrawingTool::from_array_bytes(&self.drawing_objects).unwrap()
    }

    /// Deserializes the time back from string
    pub fn from_date(&self) -> DateTime<FixedOffset> {
        DateTime::from_str(&self.from_date).unwrap()
    }

    /// Deserializes the time back from string
    pub fn to_date(&self) -> DateTime<FixedOffset> {
        DateTime::from_str(&self.to_date).unwrap()
    }

    /// Deserializes the Tz back from string
    pub fn time_zone(&self) -> Tz {
        Tz::from_str(&self.time_zone).unwrap_or_else(|_| UTC)
    }
}



