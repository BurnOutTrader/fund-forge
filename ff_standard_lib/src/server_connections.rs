use std::collections::{HashMap};
use std::str::FromStr;
use std::sync::Arc;
use heck::ToPascalCase;
use lazy_static::lazy_static;
use serde_derive::{Deserialize, Serialize};
use strum_macros::Display;
use tokio::io;
use tokio::sync::{mpsc, Mutex, RwLock};
use crate::apis::brokerage::Brokerage;
use crate::apis::vendor::DataVendor;
use crate::servers::communications_async::{ExternalReceiver, InternalReceiver, SecondaryDataReceiver, SecondaryDataSender};
use crate::servers::init_clients::{create_api_client, create_async_api_client};
use crate::servers::settings::client_settings::{get_settings_map};
use crate::servers::communications_sync::{InternalCommunicator, SecureExternalCommunicator, SynchronousCommunicator};
use crate::servers::registry_request_handlers::{registry_manage_async_requests, registry_manage_sequential_requests};
use crate::servers::request_handlers::{data_server_manage_async_requests, data_server_manage_sequential_requests};
use crate::standardized_types::data_server_messaging::FundForgeError;

/// A wrapper to allow us to pass in either a `Brokerage` or a `DataVendor`
/// # Variants
/// * `Broker(Brokerage)` - Containing a `Brokerage` object
/// * `Vendor(DataVendor)` - Containing a `DataVendor` object
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize, Debug, Display)]
pub enum ConnectionType {
    Vendor(DataVendor),
    Broker(Brokerage),
    Default,
    StrategyRegistry
}

impl FromStr for ConnectionType {
    type Err = FundForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string = s.to_pascal_case();
        match string.as_str() {
            "Default" => Ok(ConnectionType::Default),
            "StrategyRegistry" => Ok(ConnectionType::StrategyRegistry),
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

//todo, need to remove as many mutexs as possible, make all types use interior mutability.
lazy_static! {
    static ref SYNCHRONOUS_COMMUNICATORS: Arc<RwLock<HashMap<ConnectionType, Arc<SynchronousCommunicator>>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref ASYNC_OUTGOING: Arc<RwLock<HashMap<ConnectionType, Arc<SecondaryDataSender>>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref ASYNC_INCOMING: Arc<RwLock<HashMap<ConnectionType, Arc<SecondaryDataReceiver>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn get_synchronous_communicator(connection_type: ConnectionType) -> Result<Arc<SynchronousCommunicator>, String> {
    let communicators = SYNCHRONOUS_COMMUNICATORS.read().await;
    match communicators.contains_key(&connection_type) {
        true => {
            match communicators.get(&connection_type) {
                None => Err(format!("Unable to connect to server {}", connection_type)),
                Some(s) => Ok(s.clone())
            }
        },
        false => match communicators.get(&ConnectionType::Default) {
            None => Err(format!("Unable to connect to server {}", connection_type)),
            Some(s) => Ok(s.clone())
        }
    }
}

pub async fn get_async_reader(connection_type: ConnectionType) -> Result<Arc<SecondaryDataReceiver>, String> {
    let receivers = ASYNC_INCOMING.read().await;
    match receivers.contains_key(&connection_type) {
        true => {
            match receivers.get(&connection_type) {
                None => Err(format!("Unable to connect to server {}", connection_type)),
                Some(s) => Ok(s.clone())
            }
        },
        false => match receivers.get(&ConnectionType::Default) {
            None => Err(format!("Unable to connect to server {}", connection_type)),
            Some(s) => Ok(s.clone())
        }
    }
}

pub async fn get_async_sender(connection_type: ConnectionType) -> Result<Arc<SecondaryDataSender>, String> {
    let senders = ASYNC_OUTGOING.read().await;
    match senders.contains_key(&connection_type) {
        true => {
            match senders.get(&connection_type) {
                None => Err(format!("Unable to connect to server {}", connection_type)),
                Some(s) => Ok(s.clone())
            }
        },
        false => match senders.get(&ConnectionType::Default) {
            None => Err(format!("Unable to connect to server {}", connection_type)),
            Some(s) => Ok(s.clone())
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
    //initialize_strategy_registry(platform_mode.clone()).await;
    match platform_mode {
        PlatformMode::SingleMachine => {
            // setup sync and async servers for data server
            let communicator = Arc::new(SynchronousCommunicator::Channels(InternalCommunicator::new(10, 10)));
            data_server_manage_sequential_requests(communicator.clone()).await;
            let mut communicators = SYNCHRONOUS_COMMUNICATORS.write().await;
            communicators.insert(ConnectionType::Default, communicator);

            // sender simulates sending to a server, receiver simulates the server listener
            let (server_sender, server_receiver) = mpsc::channel(1000);
            // sender simulates the servers sender, receiver simulates the clients listener
            let (client_sender, client_receiver) = mpsc::channel(1000);
            let async_sender = SecondaryDataSender::InternalSender(Arc::new(client_sender));
            let async_receiver = SecondaryDataReceiver::InternalReceiver(InternalReceiver::new(server_receiver));
            data_server_manage_async_requests(Arc::new(Mutex::new(async_sender)), Arc::new(Mutex::new(async_receiver))).await;

            let mut async_senders = ASYNC_OUTGOING.write().await;
            let async_sender = SecondaryDataSender::InternalSender(Arc::new(server_sender));
            async_senders.insert(ConnectionType::Default, Arc::new(async_sender));

            let mut async_receivers = ASYNC_INCOMING.write().await;
            let async_receiver = SecondaryDataReceiver::InternalReceiver(InternalReceiver::new(client_receiver));
            async_receivers.insert(ConnectionType::Default, Arc::new(async_receiver));

            // setup sync and async servers for registry
            let communicator = Arc::new(SynchronousCommunicator::Channels(InternalCommunicator::new(1000, 1000)));
            registry_manage_sequential_requests(communicator.clone()).await;
            communicators.insert(ConnectionType::StrategyRegistry, communicator);

            // sender simulates sending to a server, receiver simulates the server listener
            let (server_sender, server_receiver) = mpsc::channel(1000);
            // sender simulates the servers sender, receiver simulates the clients listener
            let (client_sender, client_receiver) = mpsc::channel(1000);
            let async_sender = SecondaryDataSender::InternalSender(Arc::new(client_sender));
            let async_receiver = SecondaryDataReceiver::InternalReceiver(InternalReceiver::new(server_receiver));
            registry_manage_async_requests(Arc::new(Mutex::new(async_sender)), Arc::new(Mutex::new(async_receiver))).await;

            let async_sender = SecondaryDataSender::InternalSender(Arc::new(server_sender));
            async_senders.insert(ConnectionType::StrategyRegistry, Arc::new(async_sender));

            let async_receiver = SecondaryDataReceiver::InternalReceiver(InternalReceiver::new(client_receiver));
            async_receivers.insert(ConnectionType::StrategyRegistry, Arc::new(async_receiver));

            Ok(())
        },
        PlatformMode::MultiMachine => {
            let settings_map_arc = get_settings_map().clone();
            let settings_guard = settings_map_arc.lock().await;

            // for each connection type specified in our server_settings.toml we will establish a connection
            for (connection_type, settings) in settings_guard.iter() {
                //setup sync client
                let client = create_api_client(settings).await.unwrap();
                let communicator = Arc::new(SynchronousCommunicator::TlsConnections(SecureExternalCommunicator::new(Mutex::new(client))));
                SYNCHRONOUS_COMMUNICATORS.write().await.insert(connection_type.clone(), communicator);

                // set up async client
                let async_client = match create_async_api_client(&settings).await {
                    Ok(client) => client,
                    Err(e) => {
                        println!("{}", format!("Unable to establish connection to: {:?} server @ address: {:?}", connection_type, settings));
                        continue
                    }
                };
                let (read_half, write_half) = io::split(async_client);
                let async_sender = SecondaryDataSender::ExternalSender(Arc::new(Mutex::new(write_half)));
                let async_receiver = SecondaryDataReceiver::ExternalReceiver(ExternalReceiver::new(read_half));
                ASYNC_OUTGOING.write().await.insert(connection_type.clone(), Arc::new(async_sender));
                ASYNC_INCOMING.write().await.insert(connection_type.clone(), Arc::new(async_receiver));
            }
            Ok(())
        }
    }
}