use std::future::Future;
use ff_standard_lib::helpers::get_data_folder;
use ff_standard_lib::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::timeout;
use tokio_rustls::server::TlsStream;
use ff_standard_lib::server_features::database::HybridStorage;
use crate::server_side_brokerage::{account_info_response, accounts_response, commission_info_response, intraday_margin_required_response, overnight_margin_required_response, paper_account_init, live_market_order, symbol_info_response, symbol_names_response, live_enter_long, live_exit_long, live_exit_short, live_enter_short, other_orders};
use crate::server_side_datavendor::{base_data_types_response, decimal_accuracy_response, markets_response, resolutions_response, session_market_hours_response, symbols_response, tick_size_response};
use crate::stream_tasks::{deregister_streamer};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::orders::{Order, OrderRequest, OrderType, OrderUpdateEvent};
use ff_standard_lib::StreamName;
use crate::{get_shutdown_sender, stream_listener};

lazy_static!(
    pub static ref DATA_STORAGE: Arc<HybridStorage> = Arc::new(HybridStorage::new(PathBuf::from(get_data_folder()), Duration::from_secs(1800)));

    pub static ref RESPONSE_SENDERS: Arc<DashMap<StreamName, Sender<DataServerResponse>>> = Arc::new(DashMap::new());
);

pub async fn base_data_response(
    subscriptions: Vec<DataSubscription>,
    from_time: String,
    to_time: String,
    callback_id: u64,
) -> DataServerResponse {
    let from_time: DateTime<Utc> = from_time.parse().unwrap();
    let to_time: DateTime<Utc> = to_time.parse().unwrap();

    let data = match DATA_STORAGE.get_bulk_data(&subscriptions, from_time, to_time).await {
        Err(e) => return  DataServerResponse::Error { callback_id, error: FundForgeError::ServerErrorDebug(e.to_string())},
        Ok(data) => data
    };

    DataServerResponse::HistoricalBaseData {callback_id, payload: data}
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
    RESPONSE_SENDERS.insert(stream_name.clone(), response_sender.clone());

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

                    DataServerRequest::HistoricalBaseDataRange {
                        callback_id,
                        subscriptions,
                        from_time,
                        to_time
                    } => handle_callback(
                        || base_data_response(subscriptions, from_time, to_time, callback_id),
                        sender.clone()
                    ).await,

                    DataServerRequest::SymbolsVendor {
                        data_vendor,
                        market_type,
                        callback_id,
                        time
                    } => {
                        let time = match time {
                            None => None,
                            Some(t) => match DateTime::<Utc>::from_str(&t) {
                                Ok(t) => Some(t),
                                Err(_) => None
                            }
                        };
                        handle_callback(
                            || symbols_response(data_vendor, mode,stream_name, market_type, time, callback_id),
                            sender.clone()
                        ).await
                    },

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

                    DataServerRequest::IntradayMarginRequired {
                        brokerage,
                        callback_id,
                        symbol_name,
                        quantity,
                    } => handle_callback(
                        || intraday_margin_required_response(brokerage, mode, stream_name, symbol_name, quantity, callback_id),
                        sender.clone()).await,

                    DataServerRequest::OvernightMarginRequired {  brokerage,
                        callback_id,
                        symbol_name,
                        quantity
                    } => handle_callback(
                        || overnight_margin_required_response(brokerage, mode, stream_name, symbol_name, quantity, callback_id),
                        sender.clone()).await,

                    DataServerRequest::SymbolNames{ callback_id, brokerage, time } => {
                        let time = match time {
                            None => None,
                            Some(t) => match DateTime::<Utc>::from_str(&t) {
                                Ok(t) => Some(t),
                                Err(_) => None
                            }
                        };
                        handle_callback(
                            || symbol_names_response(brokerage, mode, stream_name, time, callback_id),
                            sender.clone()).await
                    }

                    DataServerRequest::CommissionInfo { callback_id, brokerage, symbol_name } => {
                        handle_callback(
                            || commission_info_response(mode, brokerage, symbol_name, stream_name, callback_id),
                            sender.clone()).await
                    }

                    DataServerRequest::SessionMarketHours { callback_id, data_vendor, symbol_name, date } => {
                        handle_callback(
                            || session_market_hours_response(mode, data_vendor, symbol_name, date, stream_name, callback_id),
                            sender.clone()).await
                    }
                    DataServerRequest::PaperAccountInit { callback_id, account_id, brokerage } => {
                        handle_callback(
                            || paper_account_init(brokerage, account_id, callback_id),
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
                        order_response(stream_name, mode, request, sender.clone()).await;
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
        deregister_streamer(&stream_name).await;
        RESPONSE_SENDERS.remove(&stream_name);
        println!("Streamer Disconnected: {}", stream_name);
    });
}

async fn response_handler(receiver: Receiver<DataServerResponse>, writer: WriteHalf<TlsStream<TcpStream>>)  {
    let mut receiver = receiver;
    let mut writer = writer;
    'receiver_loop: loop {
        match receiver.recv().await {
            Some(response) => {
                // Convert the response to bytes
                let bytes = response.to_bytes();

                // Prepare the message with a 4-byte length header in big-endian format
                let length = (bytes.len() as u64).to_be_bytes();
                let mut prefixed_msg = Vec::new();
                prefixed_msg.extend_from_slice(& length);
                prefixed_msg.extend_from_slice( & bytes);

                // Write the response to the stream
                match writer.write_all( & prefixed_msg).await {
                    Err(e) => {
                        eprintln!("Shutting down response handler {}", e);
                        break 'receiver_loop
                    }
                    Ok(_) => {}
                }
            }
            None => {
                break 'receiver_loop
            }
        }
    }
}

async fn handle_callback<F, Fut>(
    callback: F,
    sender: tokio::sync::mpsc::Sender<DataServerResponse>,
)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = DataServerResponse>,
{
    // Wrap the callback future with a timeout
    match timeout(TIMEOUT_DURATION, callback()).await {
        Ok(response) => {
            // Successfully received response, try sending it to the stream handler
            if let Err(e) = sender.send(response).await {
                // Handle send error (e.g., log it)
                println!("Failed to send response to stream handler: {:?}", e);
            }
        }
        Err(_) => {
            // Handle timeout (e.g., log it or take recovery action)
            println!("Operation timed out after {:?}", TIMEOUT_DURATION);
        }
    }
}


const TIMEOUT_DURATION: Duration = Duration::from_secs(10);
#[allow(dead_code, unused)]
async fn order_response(stream_name: StreamName, mode: StrategyMode, request: OrderRequest, sender: tokio::sync::mpsc::Sender<DataServerResponse>) {
    if mode != StrategyMode::Live {
        get_shutdown_sender().send(()).unwrap();
        panic!("Attempt to send Live order from: {:?}", mode);
    }

    async fn send_error_response(sender: &tokio::sync::mpsc::Sender<DataServerResponse>, error: OrderUpdateEvent, stream_name: &StreamName) {
        let event = DataServerResponse::OrderUpdates(error);
        if let Err(_) = sender.send(event).await {
            eprintln!("Failed to send order response to: {}", stream_name);
        }
    }

    fn create_order_rejected(order: &Order, reason: String) -> OrderUpdateEvent {
        OrderUpdateEvent::OrderRejected {
            account: order.account.clone(),
            symbol_name: order.symbol_name.clone(),
            symbol_code: "".to_string(),
            order_id: order.id.clone(),
            reason,
            tag: order.tag.clone(),
            time: Utc::now().to_string(),
        }
    }

    match request {
        OrderRequest::Create { account, order, order_type } => {
            match order_type {
                OrderType::Market => {
                    let send_order_result = timeout(TIMEOUT_DURATION, live_market_order(stream_name.clone(), mode, order.clone())).await;
                    match send_order_result {
                        Ok(Ok(_)) => {} // Order placed successfully
                        Ok(Err(e)) => {
                            send_error_response(&sender, e, &stream_name).await;
                        }
                        Err(_) => {
                            let timeout_error = create_order_rejected(&order, "Order placement timed out".to_string());
                            send_error_response(&sender, timeout_error, &stream_name).await;
                        }
                    }
                }
                OrderType::MarketIfTouched |  OrderType::StopMarket | OrderType::StopLimit | OrderType::Limit => {
                    let send_order_result = timeout(TIMEOUT_DURATION, other_orders(stream_name.clone(), mode, order.clone())).await;
                    match send_order_result {
                        Ok(Ok(_)) => {} // Order placed successfully
                        Ok(Err(e)) => {
                            send_error_response(&sender, e, &stream_name).await;
                        }
                        Err(_) => {
                            let timeout_error = create_order_rejected(&order, "Order placement timed out".to_string());
                            send_error_response(&sender, timeout_error, &stream_name).await;
                        }
                    }
                }
                OrderType::EnterLong => {
                    let send_order_result = timeout(TIMEOUT_DURATION, live_enter_long(stream_name.clone(), mode, order.clone())).await;
                    match send_order_result {
                        Ok(Ok(_)) => {} // Order placed successfully
                        Ok(Err(e)) => {
                            send_error_response(&sender, e, &stream_name).await;
                        }
                        Err(_) => {
                            let timeout_error = create_order_rejected(&order, "Order placement timed out".to_string());
                            send_error_response(&sender, timeout_error, &stream_name).await;
                        }
                    }
                }
                OrderType::EnterShort => {
                    let send_order_result = timeout(TIMEOUT_DURATION, live_enter_short(stream_name.clone(), mode, order.clone())).await;
                    match send_order_result {
                        Ok(Ok(_)) => {} // Order placed successfully
                        Ok(Err(e)) => {
                            send_error_response(&sender, e, &stream_name).await;
                        }
                        Err(_) => {
                            let timeout_error = create_order_rejected(&order, "Order placement timed out".to_string());
                            send_error_response(&sender, timeout_error, &stream_name).await;
                        }
                    }
                }
                OrderType::ExitLong => {
                    let send_order_result = timeout(TIMEOUT_DURATION, live_exit_long(stream_name.clone(), mode, order.clone())).await;
                    match send_order_result {
                        Ok(Ok(_)) => {} // Order placed successfully
                        Ok(Err(e)) => {
                            send_error_response(&sender, e, &stream_name).await;
                        }
                        Err(_) => {
                            let timeout_error = create_order_rejected(&order, "Order placement timed out".to_string());
                            send_error_response(&sender, timeout_error, &stream_name).await;
                        }
                    }
                }
                OrderType::ExitShort => {
                    let send_order_result = timeout(TIMEOUT_DURATION, live_exit_short(stream_name.clone(), mode, order.clone())).await;
                    match send_order_result {
                        Ok(Ok(_)) => {} // Order placed successfully
                        Ok(Err(e)) => {
                            send_error_response(&sender, e, &stream_name).await;
                        }
                        Err(_) => {
                            let timeout_error = create_order_rejected(&order, "Order placement timed out".to_string());
                            send_error_response(&sender, timeout_error, &stream_name).await;
                        }
                    }
                }
            }
        }
        OrderRequest::Cancel { .. } => {
            todo!()
        }
        OrderRequest::Update { .. } => {
            todo!()
        }
        OrderRequest::CancelAll { .. } => {
            todo!()
        }
        OrderRequest::FlattenAllFor { .. } => {
            todo!()
        }
    }
}
