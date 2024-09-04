use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender, SendError};
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
use crate::servers::bytes_broadcaster::{BroadCastType, BytesBroadcaster};
use crate::standardized_types::enums::StrategyMode;

lazy_static! {
    static ref LIVE_CONNECTED_STRATEGIES: Arc<RwLock<Vec<OwnerId>>> = Arc::new(RwLock::new(Vec::new()));
    static ref BACKTEST_CONNECTED_STRATEGIES: Arc<RwLock<Vec<OwnerId>>> = Arc::new(RwLock::new(Vec::new()));
    static ref LIVE_PAPER_CONNECTED_STRATEGIES: Arc<RwLock<Vec<OwnerId>>> = Arc::new(RwLock::new(Vec::new()));
    static ref GUI_BROADCATSER: BytesBroadcaster = BytesBroadcaster::new(BroadCastType::Concurrent);
    static ref STRATEGY_EVENTS_BUFFER: Arc<RwLock<HashMap<OwnerId, Arc<RwLock<BTreeMap<i64, EventTimeSlice>>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub(crate) async fn broadcast(bytes: &Vec<u8>) {
    GUI_BROADCATSER.broadcast(bytes).await;
}

pub(crate) async fn send_subscriber(id: usize, bytes: Vec<u8>) {
    GUI_BROADCATSER.send_subscriber(id, bytes).await
}

pub(crate) async fn subscribe(sender: Arc<SecondaryDataSender>) -> usize {
    GUI_BROADCATSER.subscribe(sender).await
}

pub(crate) async fn unsubscribe(id: usize) {
    GUI_BROADCATSER.unsubscribe(id).await
}

pub(crate) async fn get_live_connected_strategies() -> Vec<OwnerId> {
    LIVE_CONNECTED_STRATEGIES.read().await.clone()
}

pub(crate) async fn get_backtest_connected_strategies() -> Vec<OwnerId> {
    BACKTEST_CONNECTED_STRATEGIES.read().await.clone()
}

pub(crate) async fn get_live_paper_connected_strategies() -> Vec<OwnerId> {
    LIVE_PAPER_CONNECTED_STRATEGIES.read().await.clone()
}

pub(crate) async fn get_events_buffer() -> BTreeMap<OwnerId, BTreeMap<i64, EventTimeSlice>> {
    let buffer = STRATEGY_EVENTS_BUFFER.read().await;
    let mut return_buffer: BTreeMap<OwnerId, BTreeMap<i64, EventTimeSlice>> = BTreeMap::new();
    for (id, map) in &*buffer {
        return_buffer.insert(id.clone(), map.read().await.clone());
    }
    return_buffer
}

async fn handle_registration(owner_id: OwnerId, mode: StrategyMode) -> Result<RegistrationResponse, RegistrationResponse> {
    let registry = match mode {
        StrategyMode::Backtest => {
            BACKTEST_CONNECTED_STRATEGIES.clone()
        }
        StrategyMode::Live => {
            LIVE_CONNECTED_STRATEGIES.clone()
        }
        StrategyMode::LivePaperTrading => {
            LIVE_PAPER_CONNECTED_STRATEGIES.clone()
        }
    };
    let mut registry = registry.write().await;
    match registry.contains(&owner_id) {
        true => {
            let response = RegistrationResponse::Error(format!("Strategy registry contains an active strategy with this name: {}. Please change owner_id to run strategies in parallel", owner_id));
            Err(response)
        }
        false => {
            tokio::spawn(async move {
                let live = get_live_connected_strategies().await;
                let backtest = get_backtest_connected_strategies().await;
                let live_paper = get_live_paper_connected_strategies().await;
                broadcast(&RegistryGuiResponse::ListStrategiesResponse { live, backtest, live_paper }.to_bytes()).await;
            });
            registry.push(owner_id);
            let response = RegistrationResponse::Success;
            Ok(response)
        }
    }
}

async fn handle_disconnect(owner_id: OwnerId, mode: StrategyMode) {
    let registry = match mode {
        StrategyMode::Backtest => {
            BACKTEST_CONNECTED_STRATEGIES.clone()
        }
        StrategyMode::Live => {
            LIVE_CONNECTED_STRATEGIES.clone()
        }
        StrategyMode::LivePaperTrading => {
            LIVE_PAPER_CONNECTED_STRATEGIES.clone()
        }
    };
    registry.write().await.retain(| x | x != &owner_id );
    tokio::spawn(async move {
        let live = get_live_connected_strategies().await;
        let backtest = get_backtest_connected_strategies().await;
        let live_paper = get_live_paper_connected_strategies().await;
        broadcast(&RegistryGuiResponse::ListStrategiesResponse { live, backtest, live_paper }.to_bytes()).await;
    });
}

pub async fn handle_strategies(
    owner_id: OwnerId,
    sender: Arc<SecondaryDataSender>,
    receiver: Arc<Mutex<SecondaryDataReceiver>>,
    mode: StrategyMode
) {
    tokio::spawn(async move {
        let response =  handle_registration(owner_id.clone(), mode.clone()).await;
        match response {
            Ok(r) => {
                sender.send(&r.to_bytes()).await.unwrap();
            }
            Err(e) => {
                sender.send(&e.to_bytes()).await.unwrap();
                return;
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
                        let response = RegistryGuiResponse::StrategyEventUpdates(
                            owner_id.clone(),
                            utc_time_stamp.clone(),
                            slice,
                        );
                        broadcast(&response.to_bytes()).await
                    }
                    StrategyRequest::ShutDown(_last_time) => {
                        let response = StrategyResponse::ShutDownAcknowledged(owner_id.clone());
                        match sender.send(&response.to_bytes()).await {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                        handle_disconnect(owner_id.clone(), mode.clone()).await;
                    }
                }
            });
        }
        handle_disconnect(owner_id.clone(), mode.clone()).await;
        println! {"{} Strategy Disconnected", owner_id}
    });
}
