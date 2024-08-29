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
    static ref STRATEGY_SUBSCRIBERS: Arc<RwLock<HashMap<OwnerId, Arc<RwLock<Vec<Arc<Mutex<SecondaryDataSender>>>>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// this is used when launching in single machine
pub async fn registry_manage_async_requests(sender: Arc<Mutex<SecondaryDataSender>>, receiver: Arc<Mutex<SecondaryDataReceiver>>) {
    let mut owner_id = OwnerId::new();
    tokio::spawn(async move {
        let mut receiver = receiver.lock().await;
        while let Some(data) = receiver.receive().await {
            let request = match EventRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            match &request {
                EventRequest::RegisterStrategy(owner, ..) => {
                    CONNECTED_STRATEGIES.write().await.push(owner.clone());
                    owner_id = owner.clone();
                    println!("Registered new strategy: {:?}", owner);
                }
                EventRequest::StrategyEventUpdates(owner, slice) => {
                    //println!("Strategy updates from: {:?}", owner);
                    owner_id = owner.clone();
                    forward_to_subscribers(&owner_id, EventResponse::StrategyEventUpdates(owner.clone(), slice.clone())).await
                }
                EventRequest::ListAllStrategies => {
                    owner_id = "GUI".to_string();
                    let strategies = CONNECTED_STRATEGIES.read().await.clone();
                    match strategies.is_empty() {
                        true => { },
                        false => sender.lock().await.send(&EventResponse::ListStrategiesResponse(strategies).to_bytes()).await.unwrap()
                    }
                }
                EventRequest::Subscribe(owner) => {
                    owner_id = "GUI".to_string();
                    let mut subscribers = STRATEGY_SUBSCRIBERS.write().await;
                    if !subscribers.contains_key(owner) {
                        subscribers.insert(owner.clone(), Arc::new(RwLock::new(vec![])));
                    }
                    if let Some(subscriber_list) = subscribers.get(owner) {
                        subscriber_list.write().await.push(sender.clone());
                        println!("{:?}: subscribed to {:?}", owner_id, owner);
                        sender.lock().await.send(&EventResponse::Subscribed(owner.clone()).to_bytes()).await.unwrap();
                    }
                }
            };
        }
        if owner_id == "GUI".to_string() {
            //STRATEGY_SUBSCRIBERS.write().await.retain(|k| *k != *owner_id);
            // need a way to remove all subscriptions we subscribed to
        } else {
            let mut strategies = CONNECTED_STRATEGIES.write().await;
            strategies.retain(|k| *k != *owner_id);
            STRATEGY_SUBSCRIBERS.write().await.remove(&owner_id);
        }
        println!("Owner ID: {:?} disconnected", owner_id);
    });
}

async fn forward_to_subscribers(owner_id: &OwnerId, response: EventResponse) {
    let response_bytes: Vec<u8> = response.to_bytes();
    if let Some(subscribers_list) = STRATEGY_SUBSCRIBERS.read().await.get(owner_id) {
        let list = subscribers_list.read().await;
        for subscriber in list.iter() {
            match subscriber.lock().await.send(&response_bytes.clone()).await {
                Ok(_) => {},
                Err(e) => println!("Failed to send to subscriber: {:?} ", e),
            }
        }
    }
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
    Subscribed(OwnerId)
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
    StrategyEventUpdates(OwnerId, EventTimeSlice),
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

