use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use heck::ToPascalCase;
use lazy_static::lazy_static;
use serde_derive::{Deserialize, Serialize};
use strum_macros::Display;
use tokio::sync::Mutex;
use crate::apis::brokerage::Brokerage;
use crate::apis::vendor::DataVendor;
use crate::servers::bytes_broadcaster::BytesBroadcaster;
use crate::servers::init_clients::{create_api_client};
use crate::servers::request_handlers::manage_sequential_requests;
use crate::servers::settings::client_settings::get_settings_map;
use crate::servers::communications_sync::{InternalCommunicator, SecureExternalCommunicator, SynchronousCommunicator};
use crate::standardized_types::data_server_messaging::FundForgeError;

/// A wrapper to allow us to pass in either a `Brokerage` or a `DataVendor`
/// # Varians
/// * `Broker(Brokerage)` - Containing a `Brokerage` object
/// * `Vendor(DataVendor)` - Containing a `DataVendor` object
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize, Debug, Display)]
pub enum ConnectionType {
    Vendor(DataVendor),
    Broker(Brokerage),
    Default,
}

impl FromStr for ConnectionType {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string = s.to_pascal_case();
        match string.as_str() {
            "Default" => Ok(ConnectionType::Default),
            _ if s.starts_with("Broker:") => {
                let data = s.trim_start_matches("Broker:").trim();
                Ok(ConnectionType::Broker(Brokerage::from_str(data)?))
            },
            _ if s.starts_with("Vendor:") => {
                let data = s.trim_start_matches("Vendor:").trim();
                Ok(ConnectionType::Vendor(DataVendor::from_str(data)?))
            },
            _ => Err(FundForgeError::ClientSideErrorDebug(format!("Connection Type {} is not recognized", s))),
        }
    }
}

#[derive(Debug, Display, Clone)]
pub enum PlatformMode {
    SingleMachine,
    MultiMachine
}

lazy_static! {
    static ref SYNCHRONOUS_COMMUNICATORS: Arc<Mutex<BTreeMap<ConnectionType, Arc<SynchronousCommunicator>>>> = Arc::new(Mutex::new(BTreeMap::new()));
    static ref ASYNC_OUTGOING: Arc<Mutex<BTreeMap<ConnectionType, Arc<Mutex<BytesBroadcaster >>>>> = Arc::new(Mutex::new(BTreeMap::new()));
    static ref ASYNC_INCOMING: Arc<Mutex<BTreeMap<ConnectionType, Arc<Mutex<BytesBroadcaster >>>>> = Arc::new(Mutex::new(BTreeMap::new()));
}

pub async fn get_synchronous_communicator(connection_type: ConnectionType) -> Arc<SynchronousCommunicator> {
    let communicators = SYNCHRONOUS_COMMUNICATORS.lock().await;
    match communicators.contains_key(&connection_type) {
        true => communicators.get(&connection_type).unwrap().clone(),
        false => communicators.get(&ConnectionType::Default).unwrap().clone(),
    }
}

pub async fn get_incoming_broadcaster(connection_type: ConnectionType) -> Arc<Mutex<BytesBroadcaster>> {
    let receivers = ASYNC_INCOMING.lock().await;
    match receivers.contains_key(&connection_type) {
        true => receivers.get(&connection_type).unwrap().clone(),
        false => receivers.get(&ConnectionType::Default).unwrap().clone(),
    }
}

pub async fn get_outgoing_broadcaster(connection_type: ConnectionType) -> Arc<Mutex<BytesBroadcaster>> {
    let senders = ASYNC_OUTGOING.lock().await;
    match senders.contains_key(&connection_type) {
        true => senders.get(&connection_type).unwrap().clone(),
        false => senders.get(&ConnectionType::Default).unwrap().clone(),
    }
}

pub async fn create_broadcasters(_connection_type: ConnectionType, platform_mode: &PlatformMode) {
    match platform_mode {
        PlatformMode::SingleMachine => {
            //create an internal broadcasters, 1 for incoming, 1 for outgoing
        }
        PlatformMode::MultiMachine => {
            // create external broadcasters,  1 for incoming, 1 for outgoing

            // We need to be able to subscribe to messages from various parts of our strategy
            // It might make more sense to have a new enum which will determine where to send messages, and so we dont have to clone a ton of incoming data
            // I might need a new broadcaster type that uses dyn disopatch or impl, and make a trait for broadcast so we can take any type that implements send and receive.
            // this way it wont matter what the messages are, we can
        }
    }
}


/// Initializes clients based on the specified platform mode.
///
/// This function sets up the necessary infrastructure for communication based on whether the application
/// is running in a single machine environment or across multiple machines. For a single machine setup,
/// it creates a local sender and receiver pair. For a multi-machine setup, it utilizes a settings map
/// to create API clients for each connection type.
///
/// # Parameters
/// - `platform_mode`: A reference to the `PlatformMode` enum indicating whether the application is running
///   in a single machine or multi-machine environment.
///
/// # Returns
/// - `Result<(), FundForgeError>`: Ok(()) if the initialization is successful, otherwise an error
///   indicating what went wrong during the initialization process.
///
/// # Errors
/// This function returns an error if there is a failure in setting up the communication infrastructure,
/// such as issues creating API clients in the multi-machine setup.
/// # Examples
/// ```rust
///      use ff_standard_lib::server_connections::{initialize_clients, PlatformMode};
///
///     #[tokio::test]
///     async fn test_initialize_clients_single_machine() {
///         let result = initialize_clients(&PlatformMode::SingleMachine).await;
///         assert!(result.is_ok());
///     }
///
///     #[tokio::test]
///     async fn test_initialize_clients_multi_machine() {
///         // Assuming you have a way to mock or set up the settings for a multi-machine scenario
///         let result = initialize_clients(&PlatformMode::MultiMachine).await;
///        assert!(result.is_ok(), "ensure ff_data_server is launched for multi machine tests");
///    }
/// ```
pub async fn initialize_clients(platform_mode: &PlatformMode) -> Result<(), FundForgeError> {
    match platform_mode {
        PlatformMode::SingleMachine => {
            let communicator = Arc::new(SynchronousCommunicator::Channels(InternalCommunicator::new(100, 100)));
            manage_sequential_requests(communicator.clone()).await;
            let mut communicators = SYNCHRONOUS_COMMUNICATORS.lock().await;
            communicators.insert(ConnectionType::Default, communicator);
            Ok(())
        },
        PlatformMode::MultiMachine => {
            let settings_map_arc = get_settings_map().clone();
            let settings_guard = settings_map_arc.lock().await;
            for (connection_type, settings) in settings_guard.iter() {
                let client = create_api_client(settings).await.unwrap();
                let communicator = Arc::new(SynchronousCommunicator::TlsConnections(SecureExternalCommunicator::new(Arc::new(Mutex::new(client)))));
                SYNCHRONOUS_COMMUNICATORS.lock().await.insert(connection_type.clone(), communicator);
            }
            Ok(())
        }
    }
}