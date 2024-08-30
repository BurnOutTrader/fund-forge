use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use tokio::sync::{Mutex, RwLock};
use crate::server_connections::ConnectionType;
use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender, SendError};
use crate::servers::communications_sync::SynchronousCommunicator;
use crate::standardized_types::data_server_messaging::{AddressString, FundForgeError};
use crate::standardized_types::{OwnerId, TimeString};
use crate::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use crate::traits::bytes::Bytes;

pub async fn registry_manage_sequential_requests(communicator: Arc<SynchronousCommunicator>) {
    tokio::spawn(async move {
        while let Some(data) = communicator.receive(true).await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            println!("Parsed request: {:?}", data);
        }
    });
}

lazy_static! {
    static ref CONNECTED_STRATEGIES: Arc<RwLock<Vec<OwnerId>>> = Arc::new(RwLock::new(Vec::new()));
    static ref STRATEGY_SUBSCRIBTIONS: Arc<RwLock<HashMap<OwnerId, Arc<SecondaryDataSender>>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// this is used when launching in single machine
pub async fn registry_manage_async_requests(sender: Arc<SecondaryDataSender>, receiver: Arc<Mutex<SecondaryDataReceiver>>) {
    tokio::spawn(async move {
        let receiver = receiver.clone();
        let sender = sender;
        let binding = receiver.clone();
        let mut listener = binding.lock().await;
        'register_loop: while let Some(data) = listener.receive().await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            match request {
                EventRequest::RegisterStrategy(owner, ..) => {
                    CONNECTED_STRATEGIES.write().await.push(owner.clone());
                    handle_strategies(owner.clone(), sender, receiver).await;
                    println!("Registered Strategy: {:?}", owner);
                    break 'register_loop
                }
                EventRequest::RegisterGui => {
                    // we only support a single GUI instance to save overhead. when one connects another disconnects
                    let mut subscriptions = STRATEGY_SUBSCRIBTIONS.write().await;
                    subscriptions.clear();
                    handle_gui(sender, receiver).await;
                    println!("Registered Gui");
                    break 'register_loop
                }
                _ => {}
            };
        }
    });
}

pub async fn handle_strategies(owner_id: OwnerId, sender: Arc<SecondaryDataSender>, receiver: Arc<Mutex<SecondaryDataReceiver>>) {
    tokio::spawn(async move {
        let mut safe_shutdown = false;
        let mut  owner_id = owner_id.clone();
        let receiver = receiver.clone();
        let mut listener = receiver.lock().await;
        'register_loop: while let Some(data) = listener.receive().await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            match &request {
                EventRequest::StrategyEventUpdates(slice) => {
                    //println!("Strategy updates from: {:?}", owner);
                    forward_to_subscribers(&owner_id, EventResponse::StrategyEventUpdates(owner_id.clone(), slice.clone())).await;
                }
                _ => {}
            }
        }
        if !safe_shutdown {
            let shutdown_event = EventResponse::StrategyEventUpdates(owner_id.clone(), vec![StrategyEvent::ShutdownEvent(owner_id.clone(), String::from("Strategy Disconnect"))]);
            forward_to_subscribers(&owner_id, shutdown_event).await;
        }
        CONNECTED_STRATEGIES.write().await.retain(|x| *x != owner_id);
        println!{"{} Strategy Disconnected", owner_id}
    });
}

async fn forward_to_subscribers(owner_id: &OwnerId, response: EventResponse) {
    let response_bytes: Vec<u8> = response.to_bytes();
    if let Some(subscriber) = STRATEGY_SUBSCRIBTIONS.read().await.get(owner_id) {
        match subscriber.send(&response_bytes.clone()).await {
            Ok(_) => {},
            Err(e) => println!("Failed to send to subscriber: {:?} ", e),
        }
    }
}

pub async fn handle_gui(sender: Arc<SecondaryDataSender>, receiver: Arc<Mutex<SecondaryDataReceiver>>) {
    tokio::spawn(async move {
        let receiver = receiver.clone();
        let sender = sender.clone();
        let binding = receiver.clone();
        let mut listener = binding.lock().await;
        'register_loop: while let Some(data) = listener.receive().await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            match &request {
                EventRequest::ListAllStrategies => {
                    let strategies = CONNECTED_STRATEGIES.read().await.clone();
                    match strategies.is_empty() {
                        true => { },
                        false => sender.send(&EventResponse::ListStrategiesResponse(strategies).to_bytes()).await.unwrap()
                    }
                }
                EventRequest::Subscribe(owner) => {
                    let mut subscriptions = STRATEGY_SUBSCRIBTIONS.write().await;
                    if !subscriptions.contains_key(owner) {
                        subscriptions.insert(owner.clone(), sender.clone());
                    }
                    sender.send(&EventResponse::Subscribed(owner.clone()).to_bytes()).await.unwrap()
                }
                _ => {}
            };
        }
        let mut subscriptions = STRATEGY_SUBSCRIBTIONS.write().await;
        subscriptions.clear();
        println!{"Gui Disconnected"}
    });
}



// this is used when launching in multi-machine
/*pub async fn registry_manage_async_external(tls_stream: tokio_rustls::server::TlsStream<TcpStream>) {
    let (mut read_half, write_half) = tokio::io::split(tls_stream);
    tokio::spawn(async move {
        while let Some(data) = read_half.read(&mut []).await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            println!("Parsed request: {:?}", data);
        }
    });
}*/

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum EventResponse {
    StrategyEventUpdates(OwnerId, EventTimeSlice),
    ListStrategiesResponse(Vec<OwnerId>),
    Subscribed(OwnerId),
}

impl Bytes<Self> for EventResponse {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 100000>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<EventResponse, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<EventResponse>(archived) { //Ignore this warning: Trait `Deserialize<UiStreamResponse, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277]
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
    compare(PartialEq),
    check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum EventRequest {
    RegisterStrategy(OwnerId, TimeString),
    RegisterGui,
    StrategyEventUpdates(EventTimeSlice),
    ListAllStrategies,
    Subscribe(OwnerId),
}

impl Bytes<Self> for EventRequest {
    fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 100000>(self).unwrap();
        vec.into()
    }

    fn from_bytes(archived: &[u8]) -> Result<EventRequest, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<EventRequest>(archived) { //Ignore this warning: Trait `Deserialize<UiStreamResponse, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277] 
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}

