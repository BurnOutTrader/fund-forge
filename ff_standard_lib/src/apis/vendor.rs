use std::str::FromStr;
use std::sync::Arc;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use crate::apis::test_vendor_impl::api_client::get_test_api_client;
use crate::apis::vendor::server_responses::VendorApiResponse;
use crate::standardized_types::data_server_messaging::FundForgeError;


async fn vendor_api_object(vendor: &DataVendor) -> Arc<impl VendorApiResponse>{
    match vendor {
        DataVendor::Test => get_test_api_client().await
    }
}


#[derive(Serialize, Deserialize, Clone,Eq, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug,Hash, PartialOrd, Ord, Display)]
#[archive(
// This will generate a PartialEq impl between our unarchived and archived
// types:
compare(PartialEq),
// bytecheck can be used to validate your data if you want. To use the safe
// API, you have to derive CheckBytes for the archived type:
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// A `DataVendor` enum is a company that provides the data that is used to feed the algorithm.
/// The `DataVendor` is used to specify the data vendor that is being used to feed a `Subscription`.
/// Each `DataVendor` implements its own logic to fetch the data from the source, this logic can be modified in the `ff_data_server` crate.
pub enum DataVendor {
    Test,
}

/*impl Display for DataVendor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataVendor::Test => write!(f, "Test"),
        }
    }
}*/

impl FromStr for DataVendor {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Test" => Ok(DataVendor::Test),
            _ => Err(FundForgeError::ClientSideErrorDebug(format!("Unknown DataVendor string: {}", s))),
        }
    }
}

pub mod server_responses {
    use async_trait::async_trait;
    use crate::apis::vendor::{vendor_api_object, DataVendor};
    use crate::standardized_types::data_server_messaging::{FundForgeError, SynchronousResponseType};
    use crate::standardized_types::enums::MarketType;
    use crate::standardized_types::subscriptions::Symbol;

    /// The trait allows the server to implement the vendor specific methods for the DataVendor enum without the client needing to implement them.
    #[async_trait]
    pub trait VendorApiResponse: Sync + Send {
        async fn basedata_symbols_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError>;
        async fn resolutions_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError>;
        async fn markets_response(&self) -> Result<SynchronousResponseType, FundForgeError>;
        async fn decimal_accuracy_response(&self, symbol: Symbol) -> Result<SynchronousResponseType, FundForgeError>;
        async fn tick_size_response(&self, symbol: Symbol) -> Result<SynchronousResponseType, FundForgeError>;
    }

    /// Responses
    #[async_trait]
    impl VendorApiResponse for DataVendor {
        async fn basedata_symbols_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = vendor_api_object(self).await;
            api_client.basedata_symbols_response(market_type).await
        }

        async fn resolutions_response(&self, market_type: MarketType) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = vendor_api_object(self).await;
            api_client.resolutions_response(market_type).await
        }

        async fn markets_response(&self) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = vendor_api_object(self).await;
            api_client.markets_response().await
        }

        async fn decimal_accuracy_response(&self, symbol: Symbol) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = vendor_api_object(self).await;
            api_client.decimal_accuracy_response(symbol).await
        }

        async fn tick_size_response(&self, symbol: Symbol) -> Result<SynchronousResponseType, FundForgeError> {
            let api_client = vendor_api_object(self).await;
            api_client.tick_size_response(symbol).await
        }
    }
}

pub mod client_requests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use crate::apis::vendor::DataVendor;
    use crate::server_connections::{ConnectionType, get_synchronous_communicator};
    use crate::servers::communications_sync::SynchronousCommunicator;
    use crate::standardized_types::data_server_messaging::{FundForgeError, SynchronousRequestType, SynchronousResponseType};
    use crate::standardized_types::enums::{MarketType, Resolution};
    use crate::standardized_types::subscriptions::Symbol;

    #[async_trait]
    pub trait ClientSideDataVendor: Sync + Send {
        /// returns just the symbols available from the DataVendor in the fund forge format
        async fn symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>, FundForgeError>;
        async fn resolutions(&self, market_type: MarketType) -> Result<Vec<Resolution>, FundForgeError>;
        async fn markets(&self) -> Result<Vec<MarketType>, FundForgeError>;
        async fn decimal_accuracy(&self, symbol: Symbol) -> Result<u64, FundForgeError>;
        async fn tick_size(&self, symbol: Symbol) -> Result<f64, FundForgeError>;
    }

    #[async_trait]
    impl ClientSideDataVendor for DataVendor {
        async fn symbols(&self, market_type: MarketType) -> Result<Vec<Symbol>, FundForgeError> {
            let api_client = self.synchronous_client().await;
            let request = SynchronousRequestType::SymbolsVendor(self.clone(), market_type);
            let response = match api_client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e)
            };
            let response = SynchronousResponseType::from_bytes(&response).unwrap();
            match response {
                SynchronousResponseType::Symbols(symbols, _) => Ok(symbols),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug("Invalid response type from server".to_string()))
            }
        }

        async fn resolutions(&self, market_type: MarketType) -> Result<Vec<Resolution>, FundForgeError> {
            let api_client = self.synchronous_client().await;
            let request = SynchronousRequestType::Resolutions(self.clone(), market_type);
            let response = match api_client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e)
            };
            let response = SynchronousResponseType::from_bytes(&response).unwrap();
            match response {
                SynchronousResponseType::Resolutions(resolutions, _) => Ok(resolutions),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug("Invalid response type from server".to_string()))
            }
        }

        async fn markets(&self) -> Result<Vec<MarketType>, FundForgeError> {
            let api_client = self.synchronous_client().await;
            let request = SynchronousRequestType::Markets(self.clone());
            let response = match api_client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e)
            };
            let response = SynchronousResponseType::from_bytes(&response).unwrap();
            match response {
                SynchronousResponseType::Markets(market_type) => Ok(market_type),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug("Invalid response type from server".to_string()))
            }
        }

        async fn decimal_accuracy(&self, symbol: Symbol) -> Result<u64, FundForgeError> {
            let api_client = self.synchronous_client().await;
            let request = SynchronousRequestType::DecimalAccuracy(self.clone(), symbol);
            let response = match api_client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e)
            };
            let response = SynchronousResponseType::from_bytes(&response)?;
            match response {
                SynchronousResponseType::DecimalAccuracy(symbol, accuracy) => Ok(accuracy),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug("Invalid response type from server".to_string()))
            }
        }
        
        async fn tick_size(&self, symbol: Symbol) -> Result<f64, FundForgeError> {
            let api_client = self.synchronous_client().await;
            let request = SynchronousRequestType::TickSize(self.clone(), symbol);
            let response = match api_client.send_and_receive(request.to_bytes(), false).await {
                Ok(response) => response,
                Err(e) => return Err(e)
            };
            let response = SynchronousResponseType::from_bytes(&response)?;
            match response {
                SynchronousResponseType::TickSize(symbol, tick_size) => Ok(tick_size),
                SynchronousResponseType::Error(e) => Err(e),
                _ => Err(FundForgeError::ClientSideErrorDebug("Invalid response type from server".to_string()))
            }
        }
    }

    impl DataVendor {
        pub async fn synchronous_client(&self) -> Arc<SynchronousCommunicator> {
            get_synchronous_communicator(ConnectionType::Vendor(self.clone())).await
        }

/*        pub async fn async_receiver(&self) -> Arc<Mutex<SecondaryDataReceiver>> {
            get_readside_client(ConnectionType::Vendor(self.clone())).await
        }

        pub async fn async_sender(&self) -> Arc<Mutex<SecondaryDataSender>> {
             get_writeside_client(ConnectionType::Vendor(self.clone())).await
        }*/
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::vendor::client_requests::ClientSideDataVendor;
    use crate::server_connections::{initialize_clients, PlatformMode};
    use crate::standardized_types::enums::MarketType;

     #[tokio::test]
   async fn test_symbols_single_machine() {
         let result = initialize_clients(&PlatformMode::SingleMachine).await;
         assert!(result.is_ok());
         let vendor = DataVendor::Test;
         let symbols = vendor.symbols(MarketType::Forex).await.unwrap();
         println!("Symbols: {:?}", symbols);
         assert!(symbols.len() > 0);

         let resolutions = vendor.resolutions(MarketType::Forex).await.unwrap();
         assert!(resolutions.len() > 0);
    }

    #[tokio::test]
    async fn test_symbols_multi_machine() {
        // Assuming you have a way to mock or set up the settings for a multi-machine scenario
        let result = initialize_clients(&PlatformMode::MultiMachine).await;
        assert!(result.is_ok());

        let vendor = DataVendor::Test;
        let symbols = vendor.symbols(MarketType::Forex).await.unwrap();
        //println!("Symbols: {:?}", symbols);
        assert!(symbols.len() > 0);

        let resolutions = vendor.resolutions(MarketType::Forex).await.unwrap();
        assert!(resolutions.len() > 0);
    }
}