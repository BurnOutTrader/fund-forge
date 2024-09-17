use std::future::Future;
use ff_standard_lib::helpers::converters::load_as_bytes;
use ff_standard_lib::helpers::get_data_folder;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::data_server_messaging::{BaseDataPayload, FundForgeError, DataServerRequest, DataServerResponse, StreamRequest};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::traits::bytes::Bytes;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use ahash::AHashMap;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio_rustls::server::TlsStream;
use ff_standard_lib::apis::brokerage::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::apis::data_vendor::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::orders::orders::OrderRequest;

/// Retrieves the base data from the file system or the vendor and sends it back to the client via a NetworkMessage using the response function
pub async fn base_data_response(
    subscription: DataSubscription,
    time: String,
    callback_id: u64,
) -> DataServerResponse {
    let data_folder = PathBuf::from(get_data_folder());
    let time: DateTime<Utc> = time.parse().unwrap();

    // now we can load the data from the file system

    let file = BaseDataEnum::file_path(&data_folder, &subscription, &time).unwrap();
    //println!("file path: {:?}", file);
    let payload = if file.exists() {
        let data = load_as_bytes(file.clone()).unwrap();
        Some(BaseDataPayload {
            bytes: data,
            subscription: subscription.clone(),
        })
    } else {
        None
    };

    match payload {
        None => DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug(format!("No such file: {:?}", file))},
        Some(payload) => DataServerResponse::HistoricalBaseData{callback_id, payload}
    }
}

pub async fn base_data_many_response(
    subscriptions: Vec<DataSubscription>,
    time: String,
    callback_id: u64,
) -> DataServerResponse {
    let data_folder = PathBuf::from(get_data_folder());
    let time: DateTime<Utc> = time.parse().unwrap();

    let futures: Vec<_> = subscriptions.iter().map(|sub| {
        let time = time.clone();
        let folder = data_folder.clone();
        let sub = sub.clone();
        let time = time.clone();
        // Creating async blocks that will run concurrently
        async move {
            let file = BaseDataEnum::file_path(&folder, &sub, &time).unwrap();
            //println!("file path: {:?}", file);
            if file.exists() {
                let data = load_as_bytes(file).unwrap();
                Some(BaseDataPayload {
                    bytes: data,
                    subscription: sub.clone(),
                });
            }
            None
        }
    }).collect();

    let mut results:Vec<Option<BaseDataPayload>> = futures::future::join_all(futures).await;

    let mut payloads: Vec<BaseDataPayload> = vec![];

    for result in results.iter_mut() {
        if let Some(data) = result {
            payloads.push(data.to_owned());
        }
    }
    // return the ResponseType::HistoricalBaseData to the server fn that called this fn
    DataServerResponse::HistoricalBaseDataMany{ callback_id, payloads}
}


pub async fn data_server_manage_async_requests(
    stream: TlsStream<TcpStream>
) {
    let (read_half, write_half) = io::split(stream);
    let writer = Arc::new(Mutex::new(write_half));
    tokio::spawn(async move {
        const LENGTH: usize = 8;
        let mut receiver = read_half;
        let mut length_bytes = [0u8; LENGTH];
        let mut strategy_mode = StrategyMode::Backtest;
        while let Ok(_) = receiver.read_exact(&mut length_bytes).await {
            // Parse the length from the header
            let msg_length = u64::from_be_bytes(length_bytes) as usize;
            let mut message_body = vec![0u8; msg_length];

            // Read the message body based on the length
            match receiver.read_exact(&mut message_body).await {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error reading message body: {}", e);
                    continue;
                }
            }

            // Parse the request from the message body
            let request = match DataServerRequest::from_bytes(&message_body) {
                Ok(req) => req,
                Err(e) => {
                    eprintln!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            // Handle the request and generate a response
            match request {
                DataServerRequest::Register(register_mode) => {
                    strategy_mode = register_mode;
                },
                DataServerRequest::HistoricalBaseData {
                    callback_id,
                    subscription,
                    time,
                } => handle_callback(
                    || base_data_response(subscription, time, callback_id),
                    writer.clone()
                ).await,

                DataServerRequest::SymbolsVendor {
                    data_vendor,
                    market_type,
                    callback_id,
                } => handle_callback(
                    || data_vendor.symbols_response(market_type, callback_id),
                    writer.clone()
                ).await,

                DataServerRequest::SymbolsBroker {
                    brokerage,
                    callback_id,
                    market_type,
                } => handle_callback(
                    || brokerage.symbols_response(market_type, callback_id),
                    writer.clone()
                ).await,

                DataServerRequest::HistoricalBaseDataMany {
                    callback_id,
                    subscriptions,
                    time,
                } => handle_callback(
                    || base_data_many_response(subscriptions, time, callback_id),
                    writer.clone()
                ).await,

                DataServerRequest::Resolutions {
                    callback_id,
                    data_vendor,
                    market_type,
                } => handle_callback(
                    || data_vendor.resolutions_response(market_type, callback_id),
                    writer.clone()
                ).await,

                DataServerRequest::AccountInfo {
                    callback_id,
                    brokerage,
                    account_id,
                } => handle_callback(
                    || brokerage.account_info_response(account_id, callback_id),
                    writer.clone()
                ).await,

                DataServerRequest::Markets {
                    callback_id,
                    data_vendor,
                } => handle_callback(
                    || data_vendor.markets_response(callback_id),
                    writer.clone()
                ).await,

                DataServerRequest::TickSize {
                    callback_id,
                    data_vendor,
                    symbol_name,
                } => handle_callback(
                    || data_vendor.tick_size_response(symbol_name, callback_id),
                    writer.clone()).await,

                _ => panic!()
/*
                    DataServerRequest::StreamRequest {
                    request
                } => {
                    if strategy_mode == StrategyMode::Backtest {
                        continue
                    }
                    stream_response(request).await;
                    None
                },

                DataServerRequest::OrderRequest {
                    request
                } => {
                    if strategy_mode == StrategyMode::Backtest {
                        continue
                    }
                    order_response(request).await;
                    None
                },

                DataServerRequest::MarginRequired {
                    brokerage,
                    callback_id,
                    symbol_name,
                    quantity
                } => {
                    match strategy_mode {
                        StrategyMode::Backtest => Some(brokerage.margin_required_historical_response(symbol_name, quantity, callback_id).await),
                        StrategyMode::LivePaperTrading | StrategyMode::Live => Some(brokerage.margin_required_live_response(symbol_name, quantity, callback_id).await),
                    }
                }*/
            };
        }
    });
}

async fn handle_callback<F, Fut, T>(
    callback: F,
    writer: Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>
)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
    T: Bytes<DataServerResponse>,  // Ensures `T` can be converted to bytes for sending
{
    // Call the closure to get the future, then await the future to get the response
    let response = callback().await;

    // Convert the response to bytes
    let bytes = T::to_bytes(&response);

    // Prepare the message with a 4-byte length header in big-endian format
    let length = (bytes.len() as u64).to_be_bytes();

    // Create a buffer containing the length header followed by the response bytes
    let mut prefixed_msg = Vec::new();
    prefixed_msg.extend_from_slice(&length);
    prefixed_msg.extend_from_slice(&bytes);

    // Write the response to the stream
    let mut writer = writer.lock().await;
    match writer.write_all(&prefixed_msg).await {
        Err(e) => {
        }
        Ok(_) => {
            // Successfully wrote the message
        }
    }
}

async fn stream_response(request: StreamRequest) {

}

async fn order_response(request: OrderRequest) {

}
