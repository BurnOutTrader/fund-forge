use std::fmt::{Debug, Display};
use std::net::{SocketAddr, ToSocketAddrs};
use rkyv::{Archive, Deserialize, Serialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use crate::apis::brokerage::Brokerage;
use crate::apis::vendor::DataVendor;
use crate::standardized_types::accounts::ledgers::{AccountCurrency, AccountId, AccountInfo};
use crate::standardized_types::enums::{MarketType, Resolution};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use crate::traits::bytes::Bytes;

/// An Api key String
pub type ApiKey = String;

/// A socket Address in String format
#[derive(Clone, Serialize, Deserialize, Archive, Debug, SerdeSerialize, SerdeDeserialize, PartialEq, Eq, PartialOrd, Ord)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct AddressString(String);
impl AddressString {
    pub fn new(s: String) -> Self {
        AddressString(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    // If you want to consume the AddressString and get the inner String
    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn to_socket_addrs(&self) -> Result<SocketAddr, std::io::Error> {
        // Attempt to convert the contained string to a SocketAddr
        // Note: We're assuming only a single address needs to be parsed, not multiple.
        // This may need to be adjusted depending on your use case.
        self.0.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid socket address")
        })
    }
}

impl Display for AddressString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub enum HistoricalDataReason {}

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Represents a request type for the network message. This enum is used to specify the type of request and the returning response
///
/// # Variants
/// * [`SynchronousRequestType::HistoricalBaseData`](ff_data_vendors::networks::RequestType) : Requests the Base data for the specified subscriptions. Server returns a ResponseType::HistoricalBaseData with the data payload.
pub enum SynchronousRequestType {
    /// Requests historical `Vec<BaseDataEnum>` for a specified list of subscriptions at the specified time, this request is specifically used by the strategy backend to delegate data into the engine
    /// # Fields
    /// * `subscriptions: Vec<Subscription>`
    /// * `time: String,`
    HistoricalBaseData {
        subscriptions: DataSubscription,
        time: String,
    },

    /// Requests a list of instruments all instruments available with the `DataVendor` from the server, an instrument object is the vendors specific data type.
    /// # Fields
    /// * `DataVendor`
    /// * `MarketType`
    SymbolsVendor(DataVendor, MarketType),
    SymbolsBroker(Brokerage, MarketType),

     /// Requests a list of resolutions available with the `DataVendor` from the server
    Resolutions(DataVendor, MarketType),

    AccountCurrency(Brokerage, AccountId),

    AccountInfo(Brokerage, AccountId),

    Markets(DataVendor),

    TickSize(DataVendor, Symbol),
    
    DecimalAccuracy(DataVendor, Symbol),
}

impl SynchronousRequestType {
    pub fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }
    pub fn from_bytes(archived: &[u8]) -> Result<SynchronousRequestType, FundForgeError> {
            // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<SynchronousRequestType>(archived) { //Ignore this warning: Trait `Deserialize<RequestType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct BaseDataPayload {
    /// a Vec<BaseDataEnum> in bytes form
    pub bytes: Vec<u8>,

    /// the `Subscription` for the `bytes`
    pub subscription: DataSubscription,
}

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Represents a request type for the network message. This enum is used to specify the type of request and the returning response
pub enum SynchronousResponseType {
    /// This is for generic history requests, Responds with `payload` as `Payload` which contains:
    /// ## HistoricalBaseData Fields
    /// * `payloads` as `Vec<Payload>`
    HistoricalBaseData(Vec<BaseDataPayload>),

    /// Responds with `instruments` as `Vec<InstrumentEnum>` which contains:
    /// *  `Vec<Symbol>` for all symbols available on the server, to fullfill this the vendor will need a fn that converts from its instrument format into a `Symbol` object.
    Symbols(Vec<Symbol>, MarketType),

    /// Responds with a vec<Resolution> which represents all the native resolutions available from the vendor api (note we only support intraday resolutions, higher resolutions are consolidated by the engine)
    Resolutions(Vec<Resolution>, MarketType),
    /// Provides the client with an error message
    /// Contains a `FundForgeError` which is used to help debug and identify the type of error that occurred.
    /// [`DataServerError`](ff_data_vendors::networks::DataServerError)
    Error(FundForgeError),

    AccountCurrency(AccountId, AccountCurrency),

    AccountInfo(AccountInfo),

    Markets(Vec<MarketType>),

    TickSize(Symbol, f64),
    
    DecimalAccuracy(Symbol, u32),
}

impl SynchronousResponseType {
    pub fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }
    pub fn from_bytes(archived: &[u8]) -> Result<SynchronousResponseType, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<SynchronousResponseType>(archived) { //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, PartialEq)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Represents a response type for the network message. This is used to help debug and identify the type of error that occurred
/// # Variants
/// * `InvalidApiKey` - The vendor or broker API key used to authenticate the request is invalid. [`DataServerError::InvalidApiKey`](ff_data_vendors::networks::DataServerError)
/// * `InvalidRequestType` - The type of request being made is invalid. [`DataServerError::InvalidRequestType`](ff_data_vendors::networks::DataServerError)
/// * `ServerErrorDebug` - A server side error occurred, the debug message is provided as `String`. [`DataServerError::ServerErrorDebug`](ff_data_vendors::networks::DataServerError)
/// * `ClientSideErrorDebug` - A client side error occurred, the debug message is provided as `String`. [`DataServerError::ClientSideErrorDebug`](ff_data_vendors::networks::DataServerError)
pub enum FundForgeError {
    /// The API key used to authenticate the request is invalid.
    InvalidApiKey,
    /// The type of request being made is invalid.
    InvalidRequestType(String),
    /// A server side error occurred, the debug message is provided as `String`.
    ServerErrorDebug(String),
    /// A client side error occurred, the debug message is provided as `String`.
    ClientSideErrorDebug(String),
    /// An unknown error occurred, the blame is unknown.
    UnknownBlameError(String),
    /// An unknown error occurred, the debug message is provided as `String`.
    ConnectionNotFound(String),
}

impl Debug for FundForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundForgeError::InvalidApiKey => write!(f, "InvalidApiKey"),
            FundForgeError::InvalidRequestType(request_type) => write!(f, "InvalidRequestType: {}", request_type),
            FundForgeError::ServerErrorDebug(debug) => write!(f, "ServerErrorDebug: {}", debug),
            FundForgeError::ClientSideErrorDebug(debug) => write!(f, "ClientSideErrorDebug: {}", debug),
            FundForgeError::UnknownBlameError(debug) => write!(f, "UnknownBlameError: {}", debug),
            FundForgeError::ConnectionNotFound(debug) => write!(f, "ConnectionNotFound {}:", debug),
        }
    }
}

impl Display for FundForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundForgeError::InvalidApiKey => write!(f, "InvalidApiKey"),
            FundForgeError::InvalidRequestType(request_type) => write!(f, "InvalidRequestType: {}", request_type),
            FundForgeError::ServerErrorDebug(debug) => write!(f, "ServerErrorDebug: {}", debug),
            FundForgeError::ClientSideErrorDebug(debug) => write!(f, "ClientSideErrorDebug: {}", debug),
            FundForgeError::UnknownBlameError(debug) => write!(f, "UnknownBlameError: {}", debug),
            FundForgeError::ConnectionNotFound(debug) => write!(f, "ConnectionNotFound: {}:", debug),
        }
    }
}



#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Represents a request type for the network message. This enum is used to specify the type of request and the returning response
///
/// # Variants
/// * [`SynchronousRequestType::HistoricalBaseData`](ff_data_vendors::networks::RequestType) : Requests the Base data for the specified subscriptions. Server returns a ResponseType::HistoricalBaseData with the data payload.
pub enum AsyncRequestType {
    SubscribeLive(DataSubscription),
}

impl Bytes<Self> for AsyncRequestType {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 1024>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<AsyncRequestType, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<AsyncRequestType>(archived) { //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum AsyncResponseType {
    SubscriptionSuccess(DataSubscription),
    Error(FundForgeError),
    TimeSlice(TimeSlice),
}

impl Bytes<Self> for AsyncResponseType {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 2048>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<AsyncResponseType, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<AsyncResponseType>(archived) { //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

