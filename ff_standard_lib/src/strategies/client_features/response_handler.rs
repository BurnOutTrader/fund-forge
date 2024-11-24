use std::time::Duration;
use dashmap::DashMap;
use tokio::io::{AsyncReadExt, ReadHalf};
use tokio_rustls::TlsStream;
use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::mpsc::Sender;
use chrono::{DateTime, Utc};
use std::str::FromStr;
use crate::messages::data_server_messaging::{DataServerRequest, DataServerResponse};
use crate::standardized_types::bytes_trait::Bytes;
use crate::standardized_types::enums::StrategyMode;
use crate::standardized_types::orders::OrderUpdateEvent;
use crate::standardized_types::subscriptions::DataSubscriptionEvent;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::{live_data_receiver, request_handler};
use crate::strategies::client_features::request_handler::StrategyRequest;
use crate::strategies::client_features::server_connections::SETTINGS_MAP;
use crate::strategies::handlers::indicator_handler::IndicatorHandler;
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;
use crate::strategies::ledgers::ledger_service::LedgerService;
use crate::strategies::strategy_events::StrategyEvent;

pub async fn response_handler(
    mode: StrategyMode,
    buffer_duration: Duration,
    server_receivers: DashMap<ConnectionType, ReadHalf<TlsStream<TcpStream>>>,
    callbacks: Arc<DashMap<u64, oneshot::Sender<DataServerResponse>>>,
    order_updates_sender: Sender<(OrderUpdateEvent, DateTime<Utc>)>,
    synchronise_accounts: bool,
    strategy_event_sender: Sender<StrategyEvent>,
    ledger_service: Arc<LedgerService>, //it is better to do this than use messaging, because using a direct fn call we can concurrently update individual ledgers and have a que per ledger. sending a msg here would cause a bottleneck with more ledgers.
    indicator_handler: Arc<IndicatorHandler>,
    subscription_handler: Arc<SubscriptionHandler>,
) {
    let settings_map = SETTINGS_MAP.clone();
    for (connection, settings) in settings_map.iter() {
        let order_updates_sender = order_updates_sender.clone();
        if let Some((connection, stream)) = server_receivers.remove(connection) {
            let register_message = StrategyRequest::OneWay(connection.clone(), DataServerRequest::Register(mode.clone()));
            request_handler::send_request(register_message).await;

            let mut receiver = stream;
            let callbacks = callbacks.clone();
            let settings = settings.clone();
            let strategy_event_sender = strategy_event_sender.clone();
            let ledger_service = ledger_service.clone();
            let subscription_handler = subscription_handler.clone();
            let indicator_handler = indicator_handler.clone();
            tokio::task::spawn(async move {
                const LENGTH: usize = 8;
                let mut length_bytes = [0u8; LENGTH];
                while let Ok(_) = receiver.read_exact(&mut length_bytes).await {
                    let msg_length = u64::from_be_bytes(length_bytes) as usize;
                    let mut message_body = vec![0u8; msg_length];

                    match receiver.read_exact(&mut message_body).await {
                        Ok(_) => {},
                        Err(_) => {
                            //eprintln!("Error reading message body: {}", e);
                            continue;
                        }
                    }

                    let response = DataServerResponse::from_bytes(&message_body).unwrap();
                    match response.get_callback_id() {
                        None => {
                            match response {
                                DataServerResponse::SubscribeResponse { success, subscription, reason } => {
                                    let event = if success {
                                        DataSubscriptionEvent::Subscribed(subscription.clone())
                                    } else {
                                        DataSubscriptionEvent::FailedToSubscribe(subscription.clone(), reason.unwrap())
                                    };
                                    let event = StrategyEvent::DataSubscriptionEvent(event);
                                    match strategy_event_sender.send(event).await {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                }
                                DataServerResponse::UnSubscribeResponse { success, subscription, reason } => {
                                    let event = if success {
                                        DataSubscriptionEvent::Unsubscribed(subscription)
                                    } else {
                                        DataSubscriptionEvent::FailedUnSubscribed(subscription, reason.unwrap())
                                    };
                                    let event = StrategyEvent::DataSubscriptionEvent(event);
                                    match strategy_event_sender.send(event).await {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                }
                                DataServerResponse::OrderUpdates{ event, time} => {
                                    //println!("Event received: {}", update_event);
                                    let time = DateTime::<Utc>::from_str(&time).unwrap();
                                    match order_updates_sender.send((event, time)).await {
                                        Ok(_) => {}
                                        Err(_) => {}//eprintln!("Order Update Sender Error: {}", e)
                                    }
                                }
                                DataServerResponse::LiveAccountUpdates { account, cash_value, cash_available, cash_used } => {
                                    let ledger_service = ledger_service.clone();
                                    tokio::task::spawn(async move {
                                        ledger_service.live_account_updates(&account, cash_value, cash_available, cash_used).await;
                                    });
                                }
                                DataServerResponse::LivePositionUpdates { symbol_name, symbol_code, account, open_quantity, average_price, side, open_pnl, time } => {
                                   if synchronise_accounts {
                                       //println!("Live Position: {:?}", position);
                                        //tokio::task::spawn(async move {
                                        ledger_service.synchronize_live_position(symbol_name, symbol_code, account, open_quantity, average_price, side, open_pnl, time).await
                                        //});
                                    }
                                }
                                DataServerResponse::RegistrationResponse(port) => {
                                    //println!("Connected to server port: {}", port);
                                    if mode != StrategyMode::Backtest {
                                        live_data_receiver::handle_live_data(settings.clone(), port, buffer_duration, strategy_event_sender.clone(), ledger_service.clone(), indicator_handler.clone(), subscription_handler.clone()).await;
                                    }
                                }
                                _ => unreachable!("Incorrect response here: {:?}", response)
                            }
                        }
                        Some(id) => {
                            //eprintln!("Response with callback id: {}", id); //todo: remove this after debugging historical data deadlock
                            if let Some((_, callback_sender)) = callbacks.remove(&id) {
                                match callback_sender.send(response) {
                                    Ok(_) => {}
                                    Err(e) => eprintln!("Error sending callback: {:?}", e)
                                }
                            } else {
                                eprintln!("No callback found for id: {}", id);
                            }
                        }
                    }
                }
            });
        }
    }
}