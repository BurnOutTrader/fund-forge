use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use crate::standardized_types::strategy_events::EventTimeSlice;
use crate::standardized_types::OwnerId;
use crate::strategy_registry::guis::RegistryGuiResponse;
use crate::strategy_registry::strategies::{StrategyRequest, StrategyResponse};
use crate::strategy_registry::RegistrationResponse;
use crate::traits::bytes::Bytes;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use structopt::lazy_static::lazy_static;
use tokio::sync::{Mutex, RwLock};

lazy_static! {
    static ref CONNECTED_STRATEGIES: Arc<RwLock<Vec<OwnerId>>> = Arc::new(RwLock::new(Vec::new()));
    static ref STRATEGY_SUBSCRIBTIONS: Arc<RwLock<HashMap<OwnerId, Arc<SecondaryDataSender>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    static ref STRATEGY_EVENTS_BUFFER: Arc<RwLock<HashMap<OwnerId, Arc<RwLock<BTreeMap<i64, EventTimeSlice>>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

pub async fn get_subscribers(owner_id: &OwnerId) -> Option<Arc<SecondaryDataSender>> {
    match STRATEGY_SUBSCRIBTIONS.read().await.get(owner_id) {
        None => None,
        Some(subscriber) => Some(subscriber.clone()),
    }
}

pub async fn get_connected_strategies() -> Vec<OwnerId> {
    CONNECTED_STRATEGIES.read().await.clone()
}

pub async fn clear_subscriptions() {
    STRATEGY_SUBSCRIBTIONS.write().await.clear()
}

pub async fn subscribe_to_strategy(to_owner: OwnerId, sender: Arc<SecondaryDataSender>) {
    STRATEGY_SUBSCRIBTIONS
        .write()
        .await
        .insert(to_owner, sender);
}

pub async fn unsubscribe_from_strategy(owner: OwnerId) {
    STRATEGY_SUBSCRIBTIONS.write().await.remove(&owner);
}

pub async fn get_events_buffer(owner_id: &OwnerId) -> Option<BTreeMap<i64, EventTimeSlice>> {
    let buffer = STRATEGY_EVENTS_BUFFER.read().await;
    let buffer = buffer.get(owner_id);
    if let Some(buffer) = buffer {
        let mut buffered = buffer.write().await;
        let return_buffer = buffered.clone();
        buffered.clear();
        match return_buffer.is_empty() {
            true => None,
            false => Some(return_buffer),
        }
    } else {
        None
    }
}

pub async fn handle_strategies(
    owner_id: OwnerId,
    sender: Arc<SecondaryDataSender>,
    receiver: Arc<Mutex<SecondaryDataReceiver>>,
) {
    tokio::spawn(async move {
        if CONNECTED_STRATEGIES.write().await.contains(&owner_id) {
            let response = RegistrationResponse::Error(format!(
                "{}: strategy already registered, please rename strategy",
                owner_id.clone()
            ))
            .to_bytes();
            match sender.send(&response).await {
                Ok(_) => return,
                Err(_) => return,
            }
        } else {
            let register_response = RegistrationResponse::Success.to_bytes();
            match sender.send(&register_response).await {
                Ok(_) => {
                    println!("Registered Strategy: {:?}", owner_id);
                }
                Err(_) => return,
            }
        }

        STRATEGY_EVENTS_BUFFER
            .write()
            .await
            .insert(owner_id.clone(), Default::default());

        println!("Buffer created");

        let owner_id = owner_id.clone();
        let receiver = receiver.clone();
        let mut listener = receiver.lock().await;

        while let Some(data) = listener.receive().await {
            let owner_id = owner_id.clone();
            let sender = sender.clone();
            tokio::spawn(async move {
                let request = match StrategyRequest::from_bytes(&data) {
                    Ok(request) => request,
                    Err(_) => return,
                };
                match request {
                    StrategyRequest::StrategyEventUpdates(utc_time_stamp, slice) => {
                        forward_to_subscribers(owner_id.clone(), utc_time_stamp, slice).await;
                    }
                    StrategyRequest::ShutDown(_last_time) => {
                        let response = StrategyResponse::ShutDownAcknowledged(owner_id.clone());
                        match sender.send(&response.to_bytes()).await {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                        CONNECTED_STRATEGIES
                            .write()
                            .await
                            .retain(|x| *x != owner_id);
                        println! {"{} Strategy Disconnected", owner_id}
                    }
                }
            });
        }
    });
}

async fn forward_to_subscribers(owner_id: OwnerId, utc_time_stamp: i64, slice: EventTimeSlice) {
    match get_subscribers(&owner_id).await {
        None => {
            let buffer = STRATEGY_EVENTS_BUFFER.read().await;
            let buffer = buffer.get(&owner_id);
            if let Some(buffer) = buffer {
                let mut buffer = buffer.write().await;
                if let Some(time_slice) = buffer.get_mut(&utc_time_stamp) {
                    time_slice.extend(slice.clone());
                } else {
                    buffer.insert(utc_time_stamp.clone(), slice.clone());
                }
            }
        }
        Some(subscriber) => {
            let response = RegistryGuiResponse::StrategyEventUpdates(
                owner_id.clone(),
                utc_time_stamp.clone(),
                slice,
            )
            .to_bytes();
            match subscriber.send(&response).await {
                Ok(_) => {}
                Err(e) => println!("Failed to send to subscriber: {:?} ", e),
            }
        }
    }
}
