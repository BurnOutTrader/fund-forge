use crate::standardized_types::accounts::ledgers::{AccountId, AccountInfo, Currency};
use crate::standardized_types::enums::{MarketType, StrategyMode, SubscriptionResolutionType};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use crate::traits::bytes::Bytes;
use rkyv::{Archive, Deserialize, Serialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use std::fmt::{Debug, Display};
use std::net::{SocketAddr, ToSocketAddrs};
use rust_decimal::Decimal;
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::data_vendor::datavendor_enum::DataVendor;
use crate::standardized_types::{Price, Volume};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::orders::orders::{OrderRequest, OrderUpdateEvent};
use crate::standardized_types::symbol_info::SymbolInfo;
use crate::standardized_types::time_slices::TimeSlice;

/// An Api key String
pub type ApiKey = String;

#[derive(
    Clone,
    Serialize,
    Deserialize,
    Archive,
    Debug,
    PartialEq,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum StreamResponse {
    SubscribeBaseData(DataSubscription),
    UnSubscribeBaseData(DataSubscription),
}

#[derive(
    Clone,
    Serialize,
    Deserialize,
    Archive,
    Debug,
    SerdeSerialize,
    SerdeDeserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct AccountState {
    balance: Decimal,
    equity_used: Decimal,
    equity_available: Decimal
}

#[derive(
    Clone,
    Serialize,
    Deserialize,
    Archive,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum StreamRequest {
    AccountUpdates(Brokerage, AccountId),
    Subscribe(DataSubscription),
    Unsubscribe(DataSubscription)
}

/// A socket Address in String format
#[derive(
    Clone,
    Serialize,
    Deserialize,
    Archive,
    Debug,
    SerdeSerialize,
    SerdeDeserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash
)]
#[archive(compare(PartialEq), check_bytes)]
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
#[archive(compare(PartialEq), check_bytes, )]
#[archive_attr(derive(Debug))]
/// Represents a request type for the network message. This enum is used to specify the type of request and the returning response
///
/// # Variants
/// * [`SynchronousRequestType::HistoricalBaseData`](ff_data_vendors::networks::RequestType) : Requests the Base data for the specified subscriptions. Server returns a ResponseType::HistoricalBaseData with the data payload.
pub enum DataServerRequest {
    Register(StrategyMode),
    /// Requests historical `Vec<BaseDataEnum>` for a specified list of subscriptions at the specified time, this request is specifically used by the strategy backend to delegate data into the engine
    /// # Fields
    /// * `subscriptions: Vec<Subscription>`
    /// * `time: String,`
    HistoricalBaseData {
        callback_id: u64,
        subscription: DataSubscription,
        time: String,
    },
    /// Requests a list of instruments all instruments available with the `DataVendor` from the server, an instrument object is the vendors specific data type.
    /// # Fields
    /// * `DataVendor`
    /// * `MarketType`
    SymbolsVendor {
        callback_id: u64,
        data_vendor: DataVendor,
        market_type: MarketType
    },
    BaseDataTypes {
        callback_id: u64,
        data_vendor: DataVendor
    },
    SymbolsBroker {
        callback_id: u64,
        brokerage: Brokerage,
        market_type: MarketType
    },
    /// Requests a list of resolutions available with the `DataVendor` from the server
    Resolutions {
        callback_id: u64,
        data_vendor: DataVendor,
        market_type: MarketType
    },
    AccountInfo {
        callback_id: u64,
        brokerage: Brokerage,
        account_id: AccountId
    },
    Markets {
        callback_id: u64,
        data_vendor: DataVendor
    },
    TickSize {
        callback_id: u64,
        data_vendor: DataVendor,
        symbol_name: SymbolName
    },
    DecimalAccuracy {
        callback_id: u64,
        data_vendor: DataVendor,
        symbol_name: SymbolName
    },
    SymbolInfo{
        callback_id: u64,
        brokerage: Brokerage,
        symbol_name: SymbolName
    },
    StreamRequest {
        request: StreamRequest
    },
    OrderRequest {
        request: OrderRequest
    },
    MarginRequired {
        callback_id: u64,
        quantity: Volume,
        brokerage: Brokerage,
        symbol_name: SymbolName
    },
    Accounts{callback_id: u64, brokerage: Brokerage},
}

impl DataServerRequest {
    pub fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }
    pub fn from_bytes(archived: &[u8]) -> Result<DataServerRequest, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<DataServerRequest>(archived) {
            //Ignore this warning: Trait `Deserialize<RequestType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }
    pub fn set_callback_id(&mut self, id: u64) {
        match self {
            DataServerRequest::HistoricalBaseData { callback_id, .. } => {*callback_id = id}
            DataServerRequest::SymbolsVendor { callback_id, .. } => {*callback_id = id}
            DataServerRequest::SymbolsBroker { callback_id, .. } => {*callback_id = id}
            DataServerRequest::Resolutions {callback_id, .. } => {*callback_id = id}
            DataServerRequest::AccountInfo { callback_id, .. } => {*callback_id = id}
            DataServerRequest::BaseDataTypes { callback_id, .. } => {*callback_id = id}
            DataServerRequest::Markets { callback_id, .. } => {*callback_id = id}
            DataServerRequest::TickSize { callback_id, .. } => {*callback_id = id}
            DataServerRequest::DecimalAccuracy { callback_id, .. } => {*callback_id = id}
            DataServerRequest::SymbolInfo { callback_id, .. } => {*callback_id = id}
            DataServerRequest::StreamRequest   { .. } => {}
            DataServerRequest::Register {  .. } => {}
            DataServerRequest::OrderRequest { .. } => {}
            DataServerRequest::MarginRequired { callback_id, .. } => {*callback_id = id}
            DataServerRequest::Accounts { callback_id, .. } => {*callback_id = id}
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes, )]
#[archive_attr(derive(Debug))]
pub struct BaseDataPayload {
    /// a Vec<BaseDataEnum> in bytes form
    pub bytes: Vec<u8>,

    /// the `Subscription` for the `bytes`
    pub subscription: DataSubscription,
}

#[derive(Clone, Serialize, Deserialize, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
/// Represents a request type for the network message. This enum is used to specify the type of request and the returning response
pub enum
DataServerResponse {
    /// This is for generic history requests, Responds with `payload` as `Payload` which contains:
    /// ## HistoricalBaseData Fields
    /// * `payloads` as `BaseDataPayload`
    HistoricalBaseData {
        callback_id: u64,
        payload: BaseDataPayload
    },

    /// Responds with `instruments` as `Vec<InstrumentEnum>` which contains:
    /// *  `Vec<Symbol>` for all symbols available on the server, to fullfill this the vendor will need a fn that converts from its instrument format into a `Symbol` object.
    Symbols {
        callback_id: u64,
        symbols: Vec<Symbol>,
        market_type: MarketType
    },

    BaseDataTypes {
        callback_id: u64,
        base_data_types: Vec<BaseDataType>
    },

    /// Responds with a vec<(Resolution, BaseDataType)> which represents all the native resolutions available for the data types from the vendor api (note we only support intraday resolutions, higher resolutions are consolidated by the engine)
    Resolutions {
        callback_id: u64,
        subscription_resolutions_types: Vec<SubscriptionResolutionType>,
        market_type: MarketType
    },

    /// Provides the client with an error message
    /// Contains a `FundForgeError` which is used to help debug and identify the type of error that occurred.
    /// [`DataServerError`](ff_data_vendors::networks::DataServerError)
    Error {
        callback_id: u64,
        error: FundForgeError
    },

    AccountInfo {
        callback_id: u64,
        account_info: AccountInfo
    },

    Markets {
        callback_id: u64,
        markets: Vec<MarketType>
    },

    TickSize {
        callback_id: u64,
        tick_size: Price
    },

    DecimalAccuracy{
        callback_id: u64,
        accuracy: u8
    },

    ValuePerTick{
        callback_id: u64,
        currency: Currency,
        price: Price
    },

    SymbolInfo {
        callback_id: u64,
        symbol_info: SymbolInfo
    },

    SymbolInfoMany {
        callback_id: u64,
        info_vec: Vec<SymbolInfo>
    },

    MarginRequired {
        callback_id: u64,
        symbol_name: SymbolName,
        price: Price
    },

    SubscribeResponse {
        success: bool,
        subscription: DataSubscription,
        reason: Option<String>
    },

    UnSubscribeResponse {
        success: bool,
        subscription: DataSubscription,
        reason: Option<String>
    },
/*    AccountState(Brokerage, AccountId, AccountState),
    OrderUpdates(OrderUpdateEvent),
    PositionUpdates(PositionUpdateEvent),*/
    TimeSliceUpdates(TimeSlice),

    BaseDataUpdates(BaseDataEnum),

    Accounts{callback_id: u64, accounts: Vec<AccountId>},

    OrderUpdates(OrderUpdateEvent)
}

impl Bytes<DataServerResponse> for DataServerResponse {
    fn from_bytes(archived: &[u8]) -> Result<DataServerResponse, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<DataServerResponse>(archived) {
            //Ignore this warning: Trait `Deserialize<ResponseType, SharedDeserializeMap>` is not implemented for `ArchivedRequestType` [E0277]
            Ok(response) => Ok(response),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.into()
    }
}

impl DataServerResponse {


    pub fn get_callback_id(&self) -> Option<u64> {
        match self {
            DataServerResponse::HistoricalBaseData { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::Symbols  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::Resolutions  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::Error  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::AccountInfo  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::Markets  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::TickSize  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::DecimalAccuracy  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::ValuePerTick  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::SymbolInfo  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::SymbolInfoMany  { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::MarginRequired { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::BaseDataTypes { callback_id,.. } => Some(callback_id.clone()),
            DataServerResponse::SubscribeResponse { .. } => None,
            DataServerResponse::UnSubscribeResponse { .. } => None,
            DataServerResponse::TimeSliceUpdates(_) => None,
            DataServerResponse::Accounts {callback_id, ..} => Some(callback_id.clone()),
            DataServerResponse::BaseDataUpdates(_) => None,
            DataServerResponse::OrderUpdates(_) => None
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
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
            FundForgeError::InvalidRequestType(request_type) => {
                write!(f, "InvalidRequestType: {}", request_type)
            }
            FundForgeError::ServerErrorDebug(debug) => write!(f, "ServerErrorDebug: {}", debug),
            FundForgeError::ClientSideErrorDebug(debug) => {
                write!(f, "ClientSideErrorDebug: {}", debug)
            }
            FundForgeError::UnknownBlameError(debug) => write!(f, "UnknownBlameError: {}", debug),
            FundForgeError::ConnectionNotFound(debug) => write!(f, "ConnectionNotFound {}:", debug),
        }
    }
}

impl Display for FundForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundForgeError::InvalidApiKey => write!(f, "InvalidApiKey"),
            FundForgeError::InvalidRequestType(request_type) => {
                write!(f, "InvalidRequestType: {}", request_type)
            }
            FundForgeError::ServerErrorDebug(debug) => write!(f, "ServerErrorDebug: {}", debug),
            FundForgeError::ClientSideErrorDebug(debug) => {
                write!(f, "ClientSideErrorDebug: {}", debug)
            }
            FundForgeError::UnknownBlameError(debug) => write!(f, "UnknownBlameError: {}", debug),
            FundForgeError::ConnectionNotFound(debug) => {
                write!(f, "ConnectionNotFound: {}:", debug)
            }
        }
    }
}
