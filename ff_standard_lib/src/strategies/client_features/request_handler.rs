use tokio::sync::{mpsc, oneshot};
use dashmap::DashMap;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio_rustls::TlsStream;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse};
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::server_connections::SETTINGS_MAP;
use tokio::io::{AsyncWriteExt, WriteHalf};
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
    server_senders: DashMap<ConnectionType, WriteHalf<TlsStream<TcpStream>>>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>>,
) {
    let mut receiver = receiver;
    let callbacks_ref = callbacks.clone();
    let server_senders = server_senders;
    let settings_map = SETTINGS_MAP.clone();
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
                    let connection_type = match settings_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    if let Some(mut sender) = server_senders.get_mut(&connection_type) {
                        // Prepare the message with a 8-byte length header in big-endian format
                        let data = request.to_bytes();
                        let mut prefixed_msg = Vec::with_capacity(8 + data.len());
                        prefixed_msg.extend_from_slice(&(data.len() as u64).to_be_bytes());
                        prefixed_msg.extend_from_slice(&data);
                        // Lock the mutex to get_requests mutable access
                        if let Err(e) =  sender.value_mut().write_all(&prefixed_msg).await {
                            eprintln!("Error sending message: {:?}", e);
                        }
                    }
                }
                StrategyRequest::OneWay(connection_type, request) => {
                    let connection_type = match settings_map.contains_key(&connection_type) {
                        true => connection_type,
                        false => ConnectionType::Default
                    };
                    if let Some(mut sender) = server_senders.get_mut(&connection_type) {
                        // Prepare the message with a 8-byte length header in big-endian format
                        let data = request.to_bytes();
                        let mut prefixed_msg = Vec::with_capacity(8 + data.len());
                        prefixed_msg.extend_from_slice(&(data.len() as u64).to_be_bytes());
                        prefixed_msg.extend_from_slice(&data);
                        // Lock the mutex to get_requests mutable access
                        if let Err(e) =  sender.value_mut().write_all(&prefixed_msg).await {
                            eprintln!("Error sending message: {:?}", e);
                        }
                    }
                }
            }
        }
    });
}