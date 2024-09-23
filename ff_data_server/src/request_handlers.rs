use std::any::type_name;
use std::future::Future;
use std::net::SocketAddr;
use ff_standard_lib::helpers::converters::load_as_bytes;
use ff_standard_lib::helpers::get_data_folder;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::data_server_messaging::{BaseDataPayload, FundForgeError, DataServerRequest, DataServerResponse, StreamRequest, StreamResponse};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::traits::bytes::Bytes;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_rustls::server::TlsStream;
use ff_standard_lib::apis::brokerage::server_side_brokerage::BrokerApiResponse;
use ff_standard_lib::apis::data_vendor::datavendor_enum::DataVendor;
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
/*todo
    1. The request comes in from strategy
    2. we send request to callback fn
    3. callback calls broker/vendor fn
    4. broker/vendor api sends a outgoing request message and books a oneshot with connection_name and callback_id into its callbacks map, while callback fn awaits
    5. incoming broker/vendor message is parsed and converted to fund forge response
    6. using the connection_name and callback_id the broker/vendor read half finds the oneshot for the callback fn
    6. broker vendor fn receivers back a message on its oneshot
 */


async fn get_ip_addresses(stream: &TlsStream<TcpStream>) -> SocketAddr {
    let tcp_stream = stream.get_ref();
    tcp_stream.0.peer_addr().unwrap()
}

pub async fn data_server_manage_async_requests(
    stream: TlsStream<TcpStream>
) {
    let stream_name = get_ip_addresses(&stream).await.to_string();
    println!("stream name: {}", stream_name);
    let (read_half, write_half) = io::split(stream);
    let writer = Arc::new(Mutex::new(write_half));
    tokio::spawn(async move {
        const LENGTH: usize = 8;
        let mut receiver = read_half;
        let mut length_bytes = [0u8; LENGTH];
        let mut strategy_mode = Arc::new(RwLock::new(StrategyMode::Backtest));
        let (stream_sender, stream_receiver) = mpsc::channel(100);
        let stream_handle = stream_handler(stream_receiver, writer.clone()).await;
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
            println!("{:?}", request);
            let writer = writer.clone();
            let stream_name = stream_name.clone();
            let sender = stream_sender.clone();
            let strategy_mode = strategy_mode.clone();
            tokio::spawn(async move {
                // Handle the request and generate a response
                match request {
                    DataServerRequest::Register(register_mode) => {
                        *strategy_mode.write().await = register_mode;
                    },

                    DataServerRequest::DecimalAccuracy {
                        data_vendor,
                        callback_id,
                        symbol_name
                    } => handle_callback(
                        || data_vendor.decimal_accuracy_response(stream_name.to_string(), symbol_name, callback_id),
                        writer.clone()
                    ).await,

                    DataServerRequest::SymbolInfo {
                        symbol_name,
                        brokerage,
                        callback_id
                    } => handle_callback(
                        || brokerage.symbol_info_response(stream_name.to_string(), symbol_name, callback_id),
                        writer.clone()
                    ).await,

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
                        || data_vendor.symbols_response(stream_name.to_string(), market_type, callback_id),
                        writer.clone()
                    ).await,

                    DataServerRequest::SymbolsBroker {
                        brokerage,
                        callback_id,
                        market_type,
                    } => handle_callback(
                        || brokerage.symbols_response(stream_name.to_string(), market_type, callback_id),
                        writer.clone()
                    ).await,

                    DataServerRequest::Resolutions {
                        callback_id,
                        data_vendor,
                        market_type,
                    } => handle_callback(
                        || data_vendor.resolutions_response(stream_name.to_string(), market_type, callback_id),
                        writer.clone()
                    ).await,

                    DataServerRequest::AccountInfo {
                        callback_id,
                        brokerage,
                        account_id,
                    } => handle_callback(
                        || brokerage.account_info_response(stream_name.to_string(), account_id, callback_id),
                        writer.clone()
                    ).await,

                    DataServerRequest::Markets {
                        callback_id,
                        data_vendor,
                    } => handle_callback(
                        || data_vendor.markets_response(stream_name.to_string(), callback_id),
                        writer.clone()
                    ).await,

                    DataServerRequest::TickSize {
                        callback_id,
                        data_vendor,
                        symbol_name,
                    } => handle_callback(
                        || data_vendor.tick_size_response(stream_name.to_string(), symbol_name, callback_id),
                        writer.clone()).await,

                    DataServerRequest::MarginRequired {
                        brokerage,
                        callback_id,
                        symbol_name,
                        quantity
                    } => {
                        match *strategy_mode.read().await {
                            StrategyMode::Backtest => {
                                handle_callback(
                                    || brokerage.margin_required_historical_response(stream_name.to_string(), symbol_name, quantity, callback_id),
                                    writer.clone()).await
                            },
                            StrategyMode::LivePaperTrading | StrategyMode::Live => {
                                handle_callback(
                                    || brokerage.margin_required_live_response(stream_name.to_string(), symbol_name, quantity, callback_id),
                                    writer.clone()).await
                            },
                        }
                    },

                    DataServerRequest::StreamRequest {
                        request
                    } => {
                        if *strategy_mode.read().await == StrategyMode::Backtest {
                            eprintln!("Incorrect strategy mode for stream: {:?}", strategy_mode);
                            return
                        }
                        println!("{:?}", request);
                        handle_callback(
                            || stream_response(stream_name.clone(), request, sender),
                            writer.clone()).await
                    },

                    DataServerRequest::OrderRequest {
                        request
                    } => {
                        if *strategy_mode.read().await == StrategyMode::Backtest {
                            eprintln!("Incorrect strategy mode for orders: {:?}", strategy_mode);
                            return;
                        }
                        println!("{:?}", request);
                        order_response(request).await;
                    },
                };
            });
        }
        stream_handle.abort();
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

    //println!("Response type: {}", type_name::<T>());
    // Write the response to the stream
    let mut writer = writer.lock().await;
    match writer.write_all(&prefixed_msg).await {
        Err(e) => {
            // Handle the error (log it or take some other action)
        }
        Ok(_) => {
            // Successfully wrote the message
        }
    }
}

async fn stream_handler(receiver: Receiver<DataServerResponse>, writer: Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>) -> JoinHandle<()> {
    let mut receiver = receiver;
    tokio::spawn(async move {
        while let Some(response) = receiver.recv().await {
            // Convert the response to bytes
            let bytes = response.to_bytes();

            // Prepare the message with a 4-byte length header in big-endian format
            let length = (bytes.len() as u64).to_be_bytes();
            let mut prefixed_msg = Vec::new();
            prefixed_msg.extend_from_slice(&length);
            prefixed_msg.extend_from_slice(&bytes);

            // Write the response to the stream
            let mut writer = writer.lock().await;
            match writer.write_all(&prefixed_msg).await {
                Err(e) => {
                    // Handle the error (log it or take some other action)
                }
                Ok(_) => {
                    // Successfully wrote the message
                }
            }
        }
    })
}

async fn stream_response(stream_name: String, request: StreamRequest, sender: Sender<DataServerResponse>) -> DataServerResponse {
    match request {
        StreamRequest::AccountUpdates(_, _) => {
            panic!()
        }
        StreamRequest::Subscribe(subscription) => {
            match &subscription.symbol.data_vendor {
                DataVendor::Test => {
                    subscription.symbol.data_vendor.data_feed_subscribe(stream_name, subscription.clone(), sender).await
                }
                DataVendor::RithmicTest => {
                    panic!()
                }
            }
        }
        StreamRequest::Unsubscribe(_) => {
            panic!()
        }
    }
}

async fn order_response(request: OrderRequest) {
    match request {
        OrderRequest::Create { .. } => {}
        OrderRequest::Cancel { .. } => {}
        OrderRequest::Update { .. } => {}
    }
}
