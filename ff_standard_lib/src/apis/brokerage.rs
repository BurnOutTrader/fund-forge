use crate::apis::brokerage::server_responses::BrokerApiResponse;
use crate::apis::test_vendor_impl::api_client::get_test_api_client;
use crate::standardized_types::accounts::ledgers::{AccountCurrency, AccountId, Ledger};
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::subscriptions::{Symbol, SymbolName};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use strum_macros::Display;

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
    pub async fn margin_required(&self, symbol_name: SymbolName, quantity: u64) -> f64 { //todo make this [art of the trait
        match self {
            Brokerage::Test => quantity as f64 * 2500.0,
        }
    }

    pub async fn paper_ledger(
        &self,
        account_id: AccountId,
        currency: AccountCurrency,
        cash_value: f64,
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

    /// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
    #[async_trait]
    pub trait BrokerApiResponse: Sync + Send {
        async fn symbols_response(
            &self,
            market_type: MarketType,
        ) -> Result<SynchronousResponseType, FundForgeError>;
        async fn account_currency_reponse(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError>;

        async fn account_info_response(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError>;
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

        async fn account_currency_reponse(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = broker_api_object(self).await;
            api_client.account_currency_reponse(account_id).await
        }

        async fn account_info_response(
            &self,
            account_id: AccountId,
        ) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = broker_api_object(self).await;
            api_client.account_info_response(account_id).await
        }
    }
}

pub mod client_requests {
    use crate::apis::brokerage::Brokerage;
    use crate::server_connections::{
        get_async_reader, get_async_sender, get_synchronous_communicator, ConnectionType,
    };
    use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
    use crate::servers::communications_sync::SynchronousCommunicator;
    use crate::standardized_types::accounts::ledgers::{AccountCurrency, AccountId, Ledger};
    use crate::standardized_types::data_server_messaging::{
        FundForgeError, SynchronousRequestType, SynchronousResponseType,
    };
    use crate::standardized_types::enums::MarketType;
    use crate::standardized_types::subscriptions::Symbol;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[async_trait]
    pub trait ClientSideBrokerage: Sync + Send {
        /// returns just the symbols available from the DataVendor in the fund forge format
        async fn symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>, FundForgeError>;

        async fn account_currency(
            &self,
            account_id: AccountId,
        ) -> Result<AccountCurrency, FundForgeError>;

        async fn init_ledger(&self, account_id: AccountId) -> Result<Ledger, FundForgeError>;
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
        ) -> Result<AccountCurrency, FundForgeError> {
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
    }

    impl Brokerage {
        //ToDo: Make this function load from a toml file of currencies for the brokerage
        pub fn user_currency(&self) -> AccountCurrency {
            match self {
                Brokerage::Test => AccountCurrency::USD,
            }
        }

        pub fn starting_cash(&self) -> f64 {
            match self {
                Brokerage::Test => 100000.0,
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
