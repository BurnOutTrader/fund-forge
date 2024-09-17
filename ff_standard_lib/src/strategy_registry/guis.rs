use crate::standardized_types::data_server_messaging::{AddressString, FundForgeError};
use crate::standardized_types::strategy_events::EventTimeSlice;
use crate::traits::bytes::Bytes;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::collections::BTreeMap;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::subscriptions::DataSubscription;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum GuiRequest {
    ListAllStrategies,
    RequestBuffers
}

impl Bytes<Self> for GuiRequest {
    fn from_bytes(archived: &[u8]) -> Result<GuiRequest, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<GuiRequest>(archived) {
            //Ignore this warning: Trait `Deserialize<UiStreamResponse, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 100000>(self).unwrap();
        vec.into()
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum RegistryGuiResponse {
    StrategyEventUpdates(AddressString,i64, EventTimeSlice),
    ListStrategiesResponse{backtest: Vec<AddressString>, live: Vec<AddressString>, live_paper: Vec<AddressString>},
    StrategyAdded(AddressString, StrategyMode, Vec<DataSubscription>),
    StrategyDisconnect(AddressString),
    //Buffer {buffer: BTreeMap<AddressString, BTreeMap<i64, EventTimeSlice>> },
}

impl Bytes<Self> for RegistryGuiResponse {
    fn from_bytes(archived: &[u8]) -> Result<RegistryGuiResponse, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<RegistryGuiResponse>(archived) {
            //Ignore this warning: Trait `Deserialize<UiStreamResponse, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 100000>(self).unwrap();
        vec.into()
    }
}
