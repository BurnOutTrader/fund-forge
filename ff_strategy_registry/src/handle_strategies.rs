use ff_standard_lib::communicators::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use ff_standard_lib::standardized_types::strategy_events::StrategyEventBuffer;
use ff_standard_lib::messages::registry_messages::guis::RegistryGuiResponse;
use ff_standard_lib::messages::registry_messages::strategies::{StrategyRegistryForward, StrategyResponse};
use ff_standard_lib::messages::registry_messages::RegistrationResponse;
use ff_standard_lib::traits::bytes::Bytes;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use structopt::lazy_static::lazy_static;
use tokio::sync::{Mutex, RwLock};
use ff_standard_lib::communicators::bytes_broadcaster::{BroadCastType, BytesBroadcaster};
use ff_standard_lib::standardized_types::enums::StrategyMode;

lazy_static! {
    static ref LIVE_CONNECTED_STRATEGIES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref BACKTEST_CONNECTED_STRATEGIES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref LIVE_PAPER_CONNECTED_STRATEGIES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref GUI_BROADCATSER: BytesBroadcaster = BytesBroadcaster::new(BroadCastType::Concurrent);
    static ref STRATEGY_EVENTS_BUFFER: Arc<RwLock<HashMap<String, Arc<RwLock<BTreeMap<i64, StrategyEventBuffer >>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn broadcast(bytes: Vec<u8>) {
    GUI_BROADCATSER.broadcast(&bytes).await;
}

pub async fn send_subscriber(id: usize, bytes: Vec<u8>) {
    GUI_BROADCATSER.send_subscriber(id, bytes).await
}

pub async fn subscribe(sender: Arc<SecondaryDataSender>) -> usize {
    GUI_BROADCATSER.subscribe(sender).await
}

pub async fn unsubscribe(id: usize) {
    GUI_BROADCATSER.unsubscribe(id).await
}

pub async fn get_live_connected_strategies() -> Vec<String> {
    LIVE_CONNECTED_STRATEGIES.read().await.clone()
}

pub async fn get_backtest_connected_strategies() -> Vec<String> {
    BACKTEST_CONNECTED_STRATEGIES.read().await.clone()
}

pub async fn get_live_paper_connected_strategies() -> Vec<String> {
    LIVE_PAPER_CONNECTED_STRATEGIES.read().await.clone()
}

pub async fn get_events_buffer() -> BTreeMap<String, BTreeMap<i64, StrategyEventBuffer>> {
    let buffer = STRATEGY_EVENTS_BUFFER.read().await;
    let mut return_buffer: BTreeMap<String, BTreeMap<i64, StrategyEventBuffer>> = BTreeMap::new();
    for (id, map) in &*buffer {
        return_buffer.insert(id.clone(), map.read().await.clone());
    }
    return_buffer
}

async fn handle_registration(address_string: String, mode: StrategyMode) -> Result<RegistrationResponse, RegistrationResponse> {
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
    match registry.contains(&address_string) {
        true => {
            let response = RegistrationResponse::Error(format!("Strategy registry contains an active strategy with this name: {}. Please change owner_id to run strategies in parallel", address_string));
            Err(response)
        }
        false => {
            registry.push(address_string);
            let response = RegistrationResponse::Success;
            Ok(response)
        }
    }
}

async fn handle_disconnect(address_string: String, mode: StrategyMode) {
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
    registry.write().await.retain(| x | x != &address_string );
    let strategy_shutdown = RegistryGuiResponse::StrategyDisconnect(address_string);
    broadcast(strategy_shutdown.to_bytes()).await;
}

#[allow(unused)]
#[allow(unused_assignments)]
pub async fn handle_strategies(
    address_string: String,
    sender: Arc<SecondaryDataSender>,
    receiver: Arc<Mutex<SecondaryDataReceiver>>,
    mode: StrategyMode
) {
    tokio::spawn(async move {
        let response =  handle_registration(address_string.clone(), mode.clone()).await;
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
            .insert(address_string.clone(), Default::default());

        println!("Buffer created");

        let receiver = receiver.clone();
        let mut listener = receiver.lock().await;


        let mut good_shutdown = false;

        while let Some(data) = listener.receive().await {
            let address_string = address_string.clone();
            let sender = sender.clone();
            tokio::spawn(async move {
                let request = match StrategyRegistryForward::from_bytes(&data) {
                    Ok(request) => request,
                    Err(_) => return,
                };
                match request {
              /*      StrategyRegistryForward::StrategyEventUpdates(utc_time_stamp, slice) => {
                        let response = RegistryGuiResponse::StrategyEventUpdates(
                            address_string.clone(),
                            utc_time_stamp.clone(),
                            slice,
                        );
                        broadcast(response.to_bytes()).await
                    }*/
                    StrategyRegistryForward::ShutDown(_last_time) => {
                        let response = StrategyResponse::ShutDownAcknowledged;
                        match sender.send(&response.to_bytes()).await {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                        handle_disconnect(address_string.clone(), mode.clone()).await;


                        good_shutdown = true;
                    }
                }
            });
        }
        if !good_shutdown {
            handle_disconnect(address_string.clone(), mode.clone()).await;
        }
        println! {"{} Strategy Disconnected", address_string}
    });
}