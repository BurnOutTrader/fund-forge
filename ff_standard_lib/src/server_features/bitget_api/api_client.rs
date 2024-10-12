use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use hmac::{Hmac};
use sha2::Sha256;
use dashmap::DashMap;
use futures::stream::{SplitSink, SplitStream};
use once_cell::sync::OnceCell;
use tungstenite::Message;
use crate::communicators::internal_broadcaster::StaticInternalBroadcaster;
use crate::messages::data_server_messaging::FundForgeError;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use tokio::sync::Mutex as TokioMutex;
use crate::server_features::bitget_api::login;
use crate::server_features::bitget_api::login::BitGetCredentials;
use crate::standardized_types::subscriptions::SymbolName;
pub static BITGET_CLIENT: OnceCell<Arc<BitgetClient>> = OnceCell::new();

#[allow(unused)]
type HmacSha256 = Hmac<Sha256>;

const WEBSOCKET_URL: &str = "wss://ws.bitget.com/v2/ws/public";
#[allow(unused)]
const MAX_SUBSCRIPTIONS: usize = 1000;
#[allow(unused)]
const RECOMMENDED_SUBSCRIPTIONS: usize = 50;
#[allow(unused)]
const MESSAGE_RATE_LIMIT: usize = 10;
#[allow(unused)]
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
#[allow(unused)]
const SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(30);


#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InstType {
    Spot,
    Futures,
}
impl InstType {
    #[allow(unused)]
    fn to_string(&self) -> String {
        match self {
            InstType::Spot => "SPOT".to_string(),
            InstType::Futures => "USDT-FUTURES".to_string()
        }
    }
}

#[allow(unused)]
pub struct BitgetClient {
    credentials: BitGetCredentials,
    tick_data_write: Arc<TokioMutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    tick_subscriptions: Arc<DashMap<InstType, DashMap<SymbolName, StaticInternalBroadcaster<BaseDataEnum>>>>,

    quote_data_write: Arc<TokioMutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    quote_subscriptions: Arc<DashMap<InstType, DashMap<SymbolName, StaticInternalBroadcaster<BaseDataEnum>>>>,

    sync_socket: Arc<TokioMutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    message_queue: Arc<Mutex<VecDeque<String>>>,
    last_activity: Arc<Mutex<Instant>>,
}

impl BitgetClient {
    pub async fn new() -> Result<Self, FundForgeError> {
        let credentials = match login::get_bitget_credentials() {
            None => return Err(FundForgeError::ServerErrorDebug("BitGet credentials not found".into())),
            Some(c) => c,
        };

        // Connect and login for data feed
        let tick_subscriptions = Arc::new(DashMap::new());
        let (mut data_stream, _r) = match connect_async(WEBSOCKET_URL).await {
            Ok((stream, r)) => (stream, r),
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(format!("Failed to connect to BitGet: {}", e)))
            }
        };
        login::login(&credentials, &mut data_stream).await?;
        let (data_write, data_read) = data_stream.split();
        BitgetClient::receive_data_event_loop(data_read, tick_subscriptions.clone()).await;

        // Connect to quote stream
        let quote_subscriptions = Arc::new(DashMap::new());
        let (mut quote_data_stream, _) = match connect_async(WEBSOCKET_URL).await {
            Ok((stream, r)) => (stream, r),
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(format!("Failed to connect to BitGet: {}", e)))
            }
        };
        login::login(&credentials, &mut quote_data_stream).await?;
        let (quote_data_write, quote_data_read) = quote_data_stream.split();
        BitgetClient::receive_data_event_loop(quote_data_read, quote_subscriptions.clone()).await;

        // Connect and login for sync socket
        let (sync_socket, _) = match connect_async(WEBSOCKET_URL).await {
            Ok((stream, r)) => (stream, r),
            Err(e) => {
                return Err(FundForgeError::ServerErrorDebug(format!("Failed to connect to BitGet: {}", e)))
            }
        };

        let client = Self {
            credentials,
            tick_data_write: Arc::new(TokioMutex::new(data_write)),
            quote_data_write: Arc::new(TokioMutex::new(quote_data_write)),
            sync_socket: Arc::new(TokioMutex::new(sync_socket)),
            tick_subscriptions,
            quote_subscriptions,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            last_activity: Arc::new(Mutex::new(Instant::now())),
        };

        Ok(client)
    }

    async fn receive_data_event_loop(mut data_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, _subscriptions: Arc<DashMap<InstType, DashMap<SymbolName, StaticInternalBroadcaster<BaseDataEnum>>>>) {
        //todo learn box pin for streams and see if we can do everything from the stack.
        //todo It might be better to have a receiver for subscribe an unsubscribe and keep the broadcaster in function. use  tokio::select! to listen to stream and subscription reader at same time
        //todo need to handle sending heartbeat every 30 seconds when no incoming data
        //todo need to deserialize tick and quote data and convert to BaseDataEnum
        //todo need to broadcast base data enum to subscribers
        tokio::spawn(async move {
            while let Some(message) = data_read.next().await {
                match message {
                    Ok(msg) => {
                        if msg.is_text() {
                            let text = msg.to_text().unwrap();
                            if text == "pong" {
                                continue; // Heartbeat response
                            }
                            //todo convert incoming data and send
                        }
                    },
                    Err(e) => {
                        eprintln!("WebSocket error on data stream: {}", e);
                        // Implement reconnection logic here
                        break;
                    }
                }
            }
        });
    }

    /// Send the request and await the response
    pub async fn send_sync_request_and_receive(&self, request: &str) -> Result<String, FundForgeError> {
        let mut socket = self.sync_socket.lock().await;
        match socket.send(Message::Text(request.to_string())).await {
            Ok(_) => {}
            Err(e) => return Err(FundForgeError::ServerErrorDebug(format!("Failed to send to BitGet: {}", e)))
        };

        if let Some(response) = socket.next().await {
            match response {
                Ok(msg) => {
                    match msg {
                        Message::Text(msg) => {
                            Ok(msg)
                        }
                        Message::Binary(_) => todo!(),
                        Message::Ping(_) => todo!(),
                        Message::Pong(_) => todo!(),
                        Message::Close(_) => todo!(),
                        Message::Frame(_) => todo!(),
                    }
                },
                Err(e) => Err(FundForgeError::ServerErrorDebug(format!("{}", e))),
            }
        } else {
            Err(FundForgeError::ServerErrorDebug("No response received".into()))
        }
    }
}

