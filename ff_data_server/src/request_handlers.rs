use std::future::Future;
use ff_standard_lib::helpers::converters::load_as_bytes;
use ff_standard_lib::helpers::get_data_folder;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::messages::data_server_messaging::{BaseDataPayload, DataServerRequest, DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver};
use tokio_rustls::server::TlsStream;
use crate::server_side_brokerage::{account_info_response, accounts_response, commission_info_response, margin_required_response, symbol_info_response, symbol_names_response};
use crate::server_side_datavendor::{base_data_types_response, decimal_accuracy_response, markets_response, resolutions_response, symbols_response, tick_size_response};
use crate::stream_tasks::deregister_streamer;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::orders::OrderRequest;
use ff_standard_lib::StreamName;
use crate::stream_listener;

/// Retrieves the base data from the file system or the vendor and sends it back to the client via a NetworkMessage using the response function
pub async fn base_data_response(
    subscription: DataSubscription,
    time: String,
    callback_id: u64,
) -> DataServerResponse {
    let data_folder = PathBuf::from(get_data_folder());
    let time: DateTime<Utc> = time.parse().unwrap();

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

pub async fn manage_async_requests(
    strategy_mode: StrategyMode,
    stream: TlsStream<TcpStream>,
    stream_name: StreamName
) {
    println!("stream name: {}", stream_name);
    let (read_half, write_half) = io::split(stream);
    let strategy_mode = strategy_mode;
    let (response_sender, request_receiver) = mpsc::channel(1000);
    tokio::spawn(async move {
        response_handler(request_receiver, write_half).await;
    });
    tokio::spawn(async move {
        const LENGTH: usize = 8;
        let mut receiver = read_half;
        let mut length_bytes = [0u8; LENGTH];

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
            let stream_name = stream_name.clone();
            let mode =strategy_mode.clone();
            let sender = response_sender.clone();
            tokio::spawn(async move {
                // Handle the request and generate a response
                match request {
                    DataServerRequest::Register(_) => {

                    },
                    DataServerRequest::DecimalAccuracy {
                        data_vendor,
                        callback_id,
                        symbol_name
                    } => handle_callback(
                        || decimal_accuracy_response(data_vendor, mode, stream_name, symbol_name, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::SymbolInfo {
                        symbol_name,
                        brokerage,
                        callback_id
                    } => handle_callback(
                        || symbol_info_response(brokerage, mode, stream_name, symbol_name, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::HistoricalBaseData {
                        callback_id,
                        subscription,
                        time,
                    } => handle_callback(
                        || base_data_response(subscription, time, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::SymbolsVendor {
                        data_vendor,
                        market_type,
                        callback_id,
                    } => handle_callback(
                        || symbols_response(data_vendor, mode,stream_name, market_type, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::Resolutions {
                        callback_id,
                        data_vendor,
                        market_type,
                    } => handle_callback(
                        || resolutions_response(data_vendor, mode, stream_name, market_type, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::AccountInfo {
                        callback_id,
                        brokerage,
                        account_id,
                    } => handle_callback(
                        || account_info_response(brokerage, mode, stream_name, account_id, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::Markets {
                        callback_id,
                        data_vendor,
                    } => handle_callback(
                        || markets_response(data_vendor, mode, stream_name, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::TickSize {
                        callback_id,
                        data_vendor,
                        symbol_name,
                    } => handle_callback(
                        || tick_size_response(data_vendor, mode, stream_name, symbol_name, callback_id),
                        sender.clone()).await,

                    DataServerRequest::Accounts {
                        callback_id,
                        brokerage
                    } => handle_callback(
                        || accounts_response(brokerage, mode, stream_name, callback_id),
                        sender.clone()).await,

                    DataServerRequest::BaseDataTypes {
                        callback_id,
                        data_vendor
                    } => handle_callback(
                        || base_data_types_response(data_vendor, mode, stream_name, callback_id),
                        sender.clone()).await,

                    DataServerRequest::MarginRequired {
                        brokerage,
                        callback_id,
                        symbol_name,
                        quantity
                    } => handle_callback(
                        || margin_required_response(brokerage, mode, stream_name, symbol_name, quantity, callback_id),
                        sender.clone()).await,

                    DataServerRequest::SymbolNames{ callback_id, brokerage } => {
                        handle_callback(
                            || symbol_names_response(brokerage, mode, stream_name, callback_id),
                            sender.clone()).await
                    }

                    DataServerRequest::CommissionInfo { callback_id, brokerage, symbol_name } => {
                        handle_callback(
                            || commission_info_response(mode, brokerage, symbol_name, stream_name, callback_id),
                            sender.clone()).await
                    }


                    DataServerRequest::StreamRequest {
                        request
                    } => {
                        if mode != StrategyMode::Live && mode != StrategyMode::LivePaperTrading {
                            eprintln!("Incorrect strategy mode for stream: {:?}", strategy_mode);
                            return
                        }
                        println!("{:?}", request);
                        handle_callback(
                            || stream_listener::stream_response(stream_name, request),
                            sender.clone()).await
                    },

                    DataServerRequest::OrderRequest {
                        request
                    } => {
                        if mode != StrategyMode::Live {
                            eprintln!("Incorrect strategy mode for orders: {:?}", strategy_mode);
                            return;
                        }
                        println!("{:?}", request);
                        order_response(request).await;
                    },

                    DataServerRequest::PrimarySubscriptionFor { .. } => {
                        todo!()
                    }
                    DataServerRequest::RegisterStreamer{..} => {
                        //no need to handle here
                    }
                };
            });
        }
        deregister_streamer(&stream_name);
        println!("Shutting Down Async ReadHalf");
    });
}

async fn response_handler(receiver: Receiver<DataServerResponse>, writer: WriteHalf<TlsStream<TcpStream>>)  {
    let mut receiver = receiver;
    let mut writer = writer;
    'receiver_loop: while let Some(response) = receiver.recv().await {
        // Convert the response to bytes
        let bytes = response.to_bytes();

        // Prepare the message with a 4-byte length header in big-endian format
        let length = (bytes.len() as u64).to_be_bytes();
        let mut prefixed_msg = Vec::new();
        prefixed_msg.extend_from_slice(&length);
        prefixed_msg.extend_from_slice(&bytes);

        // Write the response to the stream
        match writer.write_all(&prefixed_msg).await {
            Err(e) => {
                eprintln!("Shutting down response handler {}", e);
                break 'receiver_loop
            }
            Ok(_) => {}
        }
    }
}

async fn handle_callback<F, Fut>(
    callback: F,
    sender: tokio::sync::mpsc::Sender<DataServerResponse>
)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = DataServerResponse>,
{
    // Call the closure to get the future, then await the future to get the response
    let response = callback().await;

    // Send the response through the channel to the stream handler
    match sender.send(response).await {
        Err(e) => {
            // Handle the error (log it or take some other action)
            println!("Failed to send response to stream handler: {:?}", e);
        }
        Ok(_) => {}
    }
}

async fn order_response(request: OrderRequest) {
    match request {
        OrderRequest::Create { .. } => {}
        OrderRequest::Cancel { .. } => {}
        OrderRequest::Update { .. } => {}
        OrderRequest::CancelAll { .. } => {}
        OrderRequest::FlattenAllFor { .. } => {}
    }
}
