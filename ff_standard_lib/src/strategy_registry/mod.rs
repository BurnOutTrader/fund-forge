use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::OwnerId;
use crate::traits::bytes::Bytes;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::standardized_types::enums::StrategyMode;

pub mod guis;
pub mod handle_gui;
pub mod handle_strategies;
pub mod strategies;
pub mod strategy_commands;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum RegistrationRequest {
    Strategy(OwnerId, StrategyMode),
    Gui,
}

impl Bytes<Self> for RegistrationRequest {
    fn from_bytes(archived: &[u8]) -> Result<RegistrationRequest, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<RegistrationRequest>(archived) {
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
pub enum RegistrationResponse {
    Success,
    Error(String),
}

impl Bytes<Self> for RegistrationResponse {
    fn from_bytes(archived: &[u8]) -> Result<RegistrationResponse, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<RegistrationResponse>(archived) {
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
