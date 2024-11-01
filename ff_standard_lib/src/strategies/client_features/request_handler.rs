use tokio::sync::{mpsc, oneshot};
use std::collections::HashMap;
use dashmap::DashMap;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use tokio::sync::mpsc::Sender;
use crate::communicators::communications_async::ExternalSender;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse};
use crate::strategies::client_features::connection_settings::client_settings::ConnectionSettings;
use crate::strategies::client_features::connection_types::ConnectionType;

pub(crate) enum StrategyRequest {
    CallBack(ConnectionType, DataServerRequest, oneshot::Sender<DataServerResponse>),
    OneWay(ConnectionType, DataServerRequest),
}

pub static DATA_SERVER_SENDER: OnceCell<Sender<StrategyRequest>> = OnceCell::new();

#[inline(always)]
pub(crate) async fn send_request(req: StrategyRequest) {
    DATA_SERVER_SENDER.get().unwrap().send(req).await.unwrap(); // Return a clone of the Arc to avoid moving the value out of the OnceCell
}

/// This response handler is also acting as a live engine.
pub async fn request_handler(
    receiver: mpsc::Receiver<StrategyRequest>,
    settings_map: HashMap<ConnectionType, ConnectionSettings>,
    server_senders: DashMap<ConnectionType, ExternalSender>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>>,
) {
    let mut receiver = receiver;
    let connection_map = settings_map.clone();
    let callbacks_ref = callbacks.clone();
    let server_senders = server_senders;
    tokio::task::spawn(async move {
        let mut callback_id_counter: u64 = 0;
        let callbacks = callbacks_ref.clone();
        while let Some(outgoing_message) = receiver.recv().await {
            match outgoing_message {
                StrategyRequest::CallBack(connection_type, mut request, oneshot) => {
                    callback_id_counter = callback_id_counter.wrapping_add(1);
                    let callbacks = callbacks.clone();
                    let id = callback_id_counter.clone();
                    callbacks.insert(id, oneshot);
                    request.set_callback_id(id.clone());
                    let connection_type = match connection_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    let sender = server_senders.get(&connection_type).unwrap();
                    sender.send(&request.to_bytes()).await;
                }
                StrategyRequest::OneWay(connection_type, request) => {
                    let connection_type = match connection_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    let sender = server_senders.get(&connection_type).unwrap();
                    sender.send(&request.to_bytes()).await;
                }
            }
        }
    });
}