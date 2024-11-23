use std::future::Future;
use ff_standard_lib::messages::data_server_messaging::{DataServerRequest, DataServerResponse, FundForgeError};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use chrono::{DateTime, Utc};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::timeout;
use tokio_rustls::server::TlsStream;
use crate::server_features::database::hybrid_storage::{DATA_STORAGE, MULTIBAR};
use crate::server_side_brokerage::{account_info_response, accounts_response, commission_info_response, live_market_order, symbol_info_response, symbol_names_response, live_enter_long, live_exit_long, live_exit_short, live_enter_short, other_orders, cancel_order, flatten_all_for, update_order, cancel_orders_on_account, exchange_rate_response, front_month_info_response};
use crate::server_side_datavendor::{base_data_types_response, decimal_accuracy_response, markets_response, resolutions_response, symbols_response, tick_size_response};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::orders::{Order, OrderRequest, OrderType, OrderUpdateEvent};
use ff_standard_lib::StreamName;
use crate::{stream_listener};
use crate::stream_tasks::deregister_streamer;

lazy_static!(
    pub static ref RESPONSE_SENDERS: Arc<DashMap<StreamName, Sender<DataServerResponse>>> = Arc::new(DashMap::new());
    //todo, we should have a disconnect broadcaster so that we broadcast on disconnect of a streamer, this needs to be done for base data response, where we await updates for before live streams
);

pub async fn compressed_file_response(
    subscriptions: Vec<DataSubscription>,
    from_time: String,
    to_time: String,
    callback_id: u64,
) -> DataServerResponse {
    let from_time = match from_time.parse::<DateTime<Utc>>() {
        Ok(t) => t,
        Err(e) => return DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ServerErrorDebug(format!("Invalid from_time: {}", e))
        }
    };
    let to_time = match to_time.parse::<DateTime<Utc>>() {
        Ok(t) => t,
        Err(e) => return DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ServerErrorDebug(format!("Invalid to_time: {}", e))
        }
    };

    // Limit the date range to prevent huge requests
    if (to_time - from_time).num_days() > 365 {
        return DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ServerErrorDebug("Date range exceeds maximum of 365 days".to_string())
        }
    }

    let data_storage = match DATA_STORAGE.get() {
        Some(storage) => storage,
        None => return DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ServerErrorDebug("Data storage not initialized".to_string())
        }
    };

    if to_time.date_naive() >= Utc::now().date_naive() {
        // Use a bounded stream for pre-subscribe tasks
        let mut tasks = FuturesUnordered::new();
        for subscription in &subscriptions {
            tasks.push(data_storage.pre_subscribe_updates(
                subscription.symbol.clone(),
                subscription.resolution,
                subscription.base_data_type
            ));
        }

        // Process with timeout
        if let Err(_) = timeout(Duration::from_secs(900), async {
            while let Some(_) = tasks.next().await {}
        }).await {
            return DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ServerErrorDebug("Pre-subscribe operations timed out".to_string())
            };
        }
    }

    match data_storage.get_compressed_files_in_range(subscriptions, from_time, to_time).await {
        Ok(data) => DataServerResponse::CompressedHistoricalData {
            callback_id,
            payload: data
        },
        Err(e) => DataServerResponse::Error {
            callback_id,
            error: FundForgeError::ServerErrorDebug(e.to_string())
        }
    }
}

pub async fn manage_async_requests(
    strategy_mode: StrategyMode,
    stream: TlsStream<TcpStream>,
    stream_name: StreamName,
) {
    println!("stream name: {}", stream_name);
    let (read_half, write_half) = io::split(stream);
    let strategy_mode = strategy_mode;
    let (response_sender, request_receiver) = mpsc::channel(1000);
    RESPONSE_SENDERS.insert(stream_name.clone(), response_sender.clone());
    let read_task = tokio::spawn(async move {
        const LENGTH: usize = 8;
        let mut receiver = read_half;
        let mut length_bytes = [0u8; LENGTH];
        let message_bar = MULTIBAR.add(ProgressBar::new_spinner());
        let bright_green_prefix = format!("\x1b[92mStrategy Connected: {}\x1b[0m", stream_name);
        // Set the colored prefix
        message_bar.set_prefix(bright_green_prefix);
        message_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {prefix} {msg}")
                .expect("Failed to set style"),
        );
        while let Ok(_) = receiver.read_exact(&mut length_bytes).await {
            let msg_length = u64::from_be_bytes(length_bytes) as usize;
            let mut message_body = vec![0u8; msg_length];

            match receiver.read_exact(&mut message_body).await {
                Ok(_) => {},
                Err(e) => {
                    let msg = format!("Error reading message body: {}", e);
                    message_bar.set_message(msg);
                    // Break the loop and stop processing on disconnection
                    break;
                }
            }

            let request = match DataServerRequest::from_bytes(&message_body) {
                Ok(req) => req,
                Err(e) => {
                    let msg = format!("Failed to parse request: {:?}", e);
                    message_bar.set_message(msg);
                    continue;
                }
            };
            let msg  = format!("Last Request: {:?}", request);
            message_bar.set_message(msg);

            let stream_name = stream_name.clone();
            let mode = strategy_mode.clone();
            let sender = response_sender.clone();

            tokio::spawn(async move {
                // Handle the request and generate a response
                match request {
                    DataServerRequest::Register(_) => {},
                    DataServerRequest::ExchangeRate {
                        callback_id,
                        from_currency,
                        to_currency,
                        date_time_string,
                        data_vendor,
                        side
                    } => {
                        handle_callback(
                            ||  exchange_rate_response(mode, from_currency, to_currency, date_time_string, data_vendor, side, callback_id),
                            sender.clone(),
                            callback_id
                        ).await
                    }
                    DataServerRequest::DecimalAccuracy {
                        data_vendor,
                        callback_id,
                        symbol_name
                    } => handle_callback(
                        || decimal_accuracy_response(data_vendor, mode, stream_name, symbol_name, callback_id),
                        sender.clone(),
                        callback_id
                    ).await,

                    DataServerRequest::SymbolInfo {
                        symbol_name,
                        brokerage,
                        callback_id
                    } => handle_callback(
                        || symbol_info_response(brokerage, mode, stream_name, symbol_name, callback_id),
                        sender.clone(),
                        callback_id
                    ).await,

                    DataServerRequest::GetCompressedHistoricalData { callback_id, subscriptions, from_time, to_time } => {
                        let time = match DateTime::<Utc>::from_str(&to_time) {
                            Ok(t) => t,
                            Err(e) => {
                                let msg = DataServerResponse::Error {
                                    callback_id,
                                    error: FundForgeError::ServerErrorDebug(format!("Invalid time format: {}", e))
                                };
                                let _ = sender.send(msg).await;
                                return;
                            }
                        };
                        match time.date_naive() >= Utc::now().date_naive() {
                            true => {
                                handle_callback_no_timeouts (
                                    || compressed_file_response(subscriptions, from_time, to_time, callback_id),
                                    sender.clone()).await
                            }
                            false => {
                                handle_callback (
                                    || compressed_file_response(subscriptions, from_time, to_time, callback_id),
                                    sender.clone(), callback_id).await
                            }
                        }
                    }

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
                            || symbols_response(data_vendor, mode, stream_name, market_type, time, callback_id),
                            sender.clone(),
                            callback_id
                        ).await
                    },

                    DataServerRequest::Resolutions {
                        callback_id,
                        data_vendor,
                        market_type,
                    } => handle_callback(
                        || resolutions_response(data_vendor, mode, stream_name, market_type, callback_id),
                        sender.clone(),
                        callback_id
                    ).await,

                    DataServerRequest::WarmUpResolutions {
                        callback_id,
                        data_vendor,
                        market_type,
                    } => handle_callback(
                        // we always use backtest mode for warmup so that way we return the resolutions we have serialized data for
                        || resolutions_response(data_vendor, StrategyMode::Backtest, stream_name, market_type, callback_id),
                        sender.clone(),
                        callback_id
                    ).await,

                    DataServerRequest::AccountInfo {
                        callback_id,
                        brokerage,
                        account_id,
                    } => handle_callback(
                        || account_info_response(brokerage, mode, stream_name, account_id, callback_id),
                        sender.clone(),
                        callback_id
                    ).await,

                    DataServerRequest::Markets {
                        callback_id,
                        data_vendor,
                    } => handle_callback(
                        || markets_response(data_vendor, mode, stream_name, callback_id),
                        sender.clone(),
                        callback_id
                    ).await,

                    DataServerRequest::TickSize {
                        callback_id,
                        data_vendor,
                        symbol_name,
                    } => handle_callback(
                        || tick_size_response(data_vendor, mode, stream_name, symbol_name, callback_id),
                        sender.clone(),callback_id).await,

                    DataServerRequest::Accounts {
                        callback_id,
                        brokerage
                    } => handle_callback(
                        || accounts_response(brokerage, mode, stream_name, callback_id),
                        sender.clone(),callback_id).await,

                    DataServerRequest::BaseDataTypes {
                        callback_id,
                        data_vendor
                    } => handle_callback(
                        || base_data_types_response(data_vendor, mode, stream_name, callback_id),
                        sender.clone(),callback_id).await,

                    DataServerRequest::SymbolNames { callback_id, brokerage, time } => {
                        let time = match time {
                            None => None,
                            Some(t) => match DateTime::<Utc>::from_str(&t) {
                                Ok(t) => Some(t),
                                Err(_) => None
                            }
                        };
                        handle_callback(
                            || symbol_names_response(brokerage, mode, stream_name, time, callback_id),
                            sender.clone(),callback_id).await
                    }

                    DataServerRequest::CommissionInfo { callback_id, brokerage, symbol_name } => {
                        handle_callback(
                            || commission_info_response(mode, brokerage, symbol_name, stream_name, callback_id),
                            sender.clone(),callback_id).await
                    }

                    DataServerRequest::FrontMonthInfo { callback_id, symbol_name, exchange, brokerage } => {
                        handle_callback(
                            || front_month_info_response(brokerage, symbol_name, exchange, stream_name, callback_id),
                            sender.clone(),callback_id).await
                    }

                    DataServerRequest::StreamRequest {
                        request
                    } => {
                        if mode != StrategyMode::Live && mode != StrategyMode::LivePaperTrading {
                            eprintln!("Incorrect strategy mode for stream: {:?}", strategy_mode);
                            return
                        }
                        //1. download latest data and await
                        //println!("{:?}", request);
                        handle_callback_no_timeouts(
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
                        //println!("{:?}", request);
                        order_response(stream_name, mode, request, sender.clone()).await;
                    },

                    DataServerRequest::PrimarySubscriptionFor { .. } => {
                        todo!()
                    }
                    DataServerRequest::RegisterStreamer { .. } => {
                        //no need to handle here
                    }
                }
            });
        }
        // Deregister when disconnected
        if strategy_mode != StrategyMode::Backtest {
            deregister_streamer(&stream_name).await;
        }
        RESPONSE_SENDERS.remove(&stream_name);
        message_bar.finish_and_clear();
    });

    // Response handler for outgoing messages
    let write_task = tokio::spawn(async move {
        response_handler(request_receiver, write_half).await;
    });

    // Wait for both tasks to complete, handle if any one task finishes first
    tokio::select! {
        _ = read_task => {
            // The read task finished (client likely disconnected)
            //println!("Read task finished for stream: {}", stream_name);
        },
        _ = write_task => {
            // The write task finished (likely due to shutdown)
            //println!("Write task finished for stream: {}", stream_name);
        }
    }
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
    callback_id: u64
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
            let response = DataServerResponse::Error {
                callback_id,
                error: FundForgeError::ServerErrorDebug("Operation timed out".to_string()),
            };
            if let Err(e) = sender.send(response).await {
                // Handle send error (e.g., log it)
                println!("Failed to send response to stream handler: {:?}", e);
            }
        }
    }
}

async fn handle_callback_no_timeouts<F, Fut>(
    callback: F,
    sender: tokio::sync::mpsc::Sender<DataServerResponse>,
)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = DataServerResponse>,
{
    // Directly await the callback future without a timeout
    let response = callback().await;

    // Try sending the response to the stream handler
    if let Err(e) = sender.send(response).await {
        // Handle send error (e.g., log it)
        println!("Failed to send response to stream handler: {:?}", e);
    }
}

async fn send_error_response(sender: &tokio::sync::mpsc::Sender<DataServerResponse>, error: OrderUpdateEvent, stream_name: &StreamName) {
    let event = DataServerResponse::OrderUpdates{event: error, time: Utc::now().to_string()};
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

const TIMEOUT_DURATION: Duration = Duration::from_secs(320);
#[allow(dead_code, unused)]
async fn order_response(stream_name: StreamName, mode: StrategyMode, request: OrderRequest, sender: tokio::sync::mpsc::Sender<DataServerResponse>) {
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
        OrderRequest::Cancel { account, order_id } => {
            cancel_order(account, order_id).await;
        }
        OrderRequest::Update { account, order_id, update } => {
            let send_order_result = timeout(TIMEOUT_DURATION, update_order(account.clone(), order_id.clone(), update)).await;
            match send_order_result {
                Ok(Ok(_)) => {} // Order placed successfully
                Ok(Err(e)) => {
                    send_error_response(&sender, e, &stream_name).await;
                }
                Err(_) => {
                    let timeout_error = OrderUpdateEvent::OrderUpdateRejected {
                        account,
                        order_id,
                        reason: "Order update timed out".to_string(),
                        time: Utc::now().to_string(),
                    };
                    send_error_response(&sender, timeout_error, &stream_name).await;
                }
            }
        }
        OrderRequest::CancelAll { account } => {
            cancel_orders_on_account(account).await;
        }
        OrderRequest::FlattenAllFor { account } => {
            flatten_all_for(account).await;
        }
    }
}
