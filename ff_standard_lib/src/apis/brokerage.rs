use crate::apis::brokerage::server_responses::BrokerApiResponse;
use crate::apis::test_vendor_impl::api_client::get_test_api_client;
use crate::standardized_types::accounts::ledgers::{AccountId, Currency, Ledger};
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::subscriptions::SymbolName;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use rust_decimal_macros::dec;
use strum_macros::Display;
use crate::helpers::decimal_calculators::round_to_decimals;
use crate::standardized_types::{Price, Volume};
use crate::standardized_types::accounts::position::Position;
use crate::traits::bytes::Bytes;

async fn broker_api_object(vendor: &Brokerage) -> Arc<impl BrokerApiResponse> {
    match vendor {
        Brokerage::Test => get_test_api_client().await,
    }
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Eq,
    Serialize_rkyv,
    Deserialize_rkyv,
    Archive,
    PartialEq,
    Debug,
    Hash,
    PartialOrd,
    Ord,
    Display,
)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum Brokerage {
    Test,
}

impl Brokerage {
    pub async fn margin_required_historical(&self, _symbol_name: SymbolName, quantity: Volume) -> Price { //todo make this [art of the trait
        match self {
            Brokerage::Test => round_to_decimals(quantity * dec!(1000), 2)
        }
    }

    pub async fn paper_ledger(
        &self,
        account_id: AccountId,
        currency: Currency,
        cash_value: Price,
    ) -> Ledger {
        match self {
            Brokerage::Test => {
                Ledger::paper_account_init(account_id, Brokerage::Test, cash_value, currency)
            }
        }
    }
}

impl FromStr for Brokerage {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Test" => Ok(Brokerage::Test),
            _ => Err(FundForgeError::ClientSideErrorDebug(format!(
                "Invalid brokerage string: {}",
                s
            ))),
        }
    }
}

pub mod server_responses {
    use crate::apis::brokerage::{broker_api_object, Brokerage};
    use crate::standardized_types::accounts::ledgers::AccountId;
    use crate::standardized_types::data_server_messaging::{
        FundForgeError, SynchronousResponseType,
    };
    use crate::standardized_types::enums::MarketType;
    use async_trait::async_trait;
    use crate::standardized_types::subscriptions::SymbolName;

    /// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
    #[async_trait]
    pub trait BrokerApiResponse: Sync + Send {
        async fn symbols_response(
            &self,
            market_type: MarketType,
        ) -> Result<SynchronousResponseType, FundForgeError>;
        async fn account_currency_response(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError>;

        async fn account_info_response(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError>;

        async fn symbol_info_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError>;
    }

    /// Responses
    #[async_trait]
    impl BrokerApiResponse for Brokerage {
        async fn symbols_response(
            &self,
            market_type: MarketType,
        ) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = broker_api_object(self).await;
            api_client.symbols_response(market_type).await
        }

        async fn account_currency_response(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = broker_api_object(self).await;
            api_client.account_currency_response(account_id).await
        }

        async fn account_info_response(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = broker_api_object(self).await;
            api_client.account_info_response(account_id).await
        }

        async fn symbol_info_response(&self, symbol_name: SymbolName) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = broker_api_object(self).await;
            api_client.symbol_info_response(symbol_name).await
        }
    }
}

pub mod client_requests {
    use crate::apis::brokerage::{Brokerage, SymbolInfo};
    use crate::server_connections::{
        get_async_reader, get_async_sender, get_synchronous_communicator, ConnectionType,
    };
    use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
    use crate::servers::communications_sync::SynchronousCommunicator;
    use crate::standardized_types::accounts::ledgers::{AccountId, Currency, Ledger};
    use crate::standardized_types::data_server_messaging::{
        FundForgeError, SynchronousRequestType, SynchronousResponseType,
    };
    use crate::standardized_types::enums::MarketType;
    use crate::standardized_types::subscriptions::{Symbol, SymbolName};
    use async_trait::async_trait;
    use std::sync::Arc;
    use rust_decimal_macros::dec;
    use tokio::sync::Mutex;
    use crate::standardized_types::Price;

    #[async_trait]
    pub trait ClientSideBrokerage: Sync + Send {
        /// returns just the symbols available from the DataVendor in the fund forge format
        async fn symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>, FundForgeError>;

        async fn account_currency(
            &self,
            account_id: AccountId,
        ) -> Result<Currency, FundForgeError>;

        async fn init_ledger(&self, account_id: AccountId) -> Result<Ledger, FundForgeError>;

        async fn symbol_info(&self, symbol_name: SymbolName) -> Result<SymbolInfo, FundForgeError>;
    }

    #[async_trait]
    impl ClientSideBrokerage for Brokerage {
        async fn symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>, FundForgeError> {
            let api_client = self.synchronous_client().await;
            let request = SynchronousRequestType::SymbolsBroker(self.clone(), market_type);
            let response = match api_client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e),
            };
            let response = SynchronousResponseType::from_bytes(&response).unwrap();
            match response {
                SynchronousResponseType::Symbols(symbols, _) => Ok(symbols),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug(
                    "Invalid response type from server".to_string(),
                )),
            }
        }
        async fn account_currency(
            &self,
            account_id: AccountId,
        ) -> Result<Currency, FundForgeError> {
            let client = self.synchronous_client().await;
            let request = SynchronousRequestType::AccountCurrency(self.clone(), account_id);
            let response = match client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e),
            };
            let response = SynchronousResponseType::from_bytes(&response).unwrap();
            match response {
                SynchronousResponseType::AccountCurrency(_, currency) => Ok(currency),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug(
                    "Invalid response type from server".to_string(),
                )),
            }
        }

        async fn init_ledger(&self, account_id: AccountId) -> Result<Ledger, FundForgeError> {
            let client = self.synchronous_client().await;
            let request = SynchronousRequestType::AccountInfo(self.clone(), account_id);
            let response = match client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e),
            };
            let response = match SynchronousResponseType::from_bytes(&response) {
                Ok(response) => response,
                Err(e) => return Err(e),
            };
            match response {
                SynchronousResponseType::AccountInfo(account_info) => Ok(Ledger::new(account_info)),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug(
                    "Invalid response type from server".to_string(),
                )),
            }
        }

        async fn symbol_info(&self, symbol_name: SymbolName) -> Result<SymbolInfo, FundForgeError> {
            let api_client = self.synchronous_client().await;
            let request = SynchronousRequestType::SymbolInfo(self.clone(), symbol_name);
            let response = match api_client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e),
            };
            let response = SynchronousResponseType::from_bytes(&response)?;
            match response {
                SynchronousResponseType::SymbolInfo(info) => Ok(info),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug(
                    "Invalid response type from server".to_string(),
                )),
            }
        }
    }

    impl Brokerage {
        //ToDo: Make this function load from a toml file of currencies for the brokerage
        pub fn user_currency(&self) -> Currency {
            match self {
                Brokerage::Test => Currency::USD,
            }
        }

        pub fn starting_cash(&self) -> Price {
            match self {
                Brokerage::Test => dec!(100000.0),
            }
        }

        pub async fn synchronous_client(&self) -> Arc<SynchronousCommunicator> {
            match get_synchronous_communicator(ConnectionType::Broker(self.clone())).await {
                Err(e) => panic!("{}", e),
                Ok(s) => s,
            }
        }

        pub async fn async_receiver(&self) -> Arc<Mutex<SecondaryDataReceiver>> {
            match get_async_reader(ConnectionType::Broker(self.clone())).await {
                Err(e) => panic!("{}", e),
                Ok(s) => s,
            }
        }

        pub async fn async_sender(&self) -> Arc<SecondaryDataSender> {
            match get_async_sender(ConnectionType::Broker(self.clone())).await {
                Err(e) => panic!("{}", e),
                Ok(s) => s,
            }
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct SymbolInfo {
    symbol_name: SymbolName,
    pub(crate) pnl_currency: Currency,
    pub(crate) value_per_tick: Price,
    pub(crate) tick_size: Price
}

impl SymbolInfo {
    pub fn new(symbol_name: SymbolName,
               pnl_currency: Currency,
               value_per_tick: Price,
               tick_size: Price) -> Self {
        Self {
            symbol_name,
            pnl_currency,
            value_per_tick,
            tick_size
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct AccountInfo {
    pub account_id: AccountId,
    pub brokerage: Brokerage,
    pub cash_value: Price,
    pub cash_available: Price,
    pub currency: Currency,
    pub cash_used: Price,
    pub positions: Vec<Position>,
    pub positions_closed: Vec<Position>,
    pub is_hedging: bool,
}

impl Bytes<Self> for AccountInfo {
    fn from_bytes(archived: &[u8]) -> Result<AccountInfo, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<AccountInfo>(archived) {
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