use std::sync::Arc;
#[allow(unused_imports)]
use std::time::Duration;
use async_std::stream::StreamExt;
#[allow(unused_imports)]
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::RequestLogout;
use futures::SinkExt;
use prost::Message as ProstMessage;
#[allow(unused_imports)]
use tokio::time::sleep;
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use crate::rithmic_api::api_client::{RithmicBrokerageClient};
use futures::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use crate::request_handlers::RESPONSE_SENDERS;
use crate::rithmic_api::client_base::api_base::extract_template_id;
use crate::rithmic_api::client_base::errors::RithmicApiError;
use crate::rithmic_api::client_base::rithmic_proto_objects::rti::request_login::SysInfraType;
use crate::rithmic_api::plant_handlers::handle_history_plant::match_history_plant_id;
use crate::rithmic_api::plant_handlers::handle_order_plant::match_order_plant_id;
use crate::rithmic_api::plant_handlers::handle_pnl_plant::match_pnl_plant_id;
use crate::rithmic_api::plant_handlers::handle_repo_plant::match_repo_plant_id;
use crate::rithmic_api::plant_handlers::handle_tick_plant::match_ticker_plant_id;
use crate::rithmic_api::plant_handlers::reconnect::attempt_reconnect;
use crate::subscribe_server_shutdown;
use tokio::sync::mpsc;
use tokio::task;

pub fn handle_rithmic_responses(
    client: Arc<RithmicBrokerageClient>,
    mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    plant: SysInfraType,
) {
    let mut shutdown_receiver = subscribe_server_shutdown();
    let mut length_buf = [0u8; 4];

    // Use bounded channel with backpressure
    let (tx, mut rx) = mpsc::channel(10000);

    // Spawn message processor task
    let process_client = client.clone();
    let process_plant = plant.clone();
    tokio::spawn(async move {
        while let Some((template_id, message)) = rx.recv().await {
            match process_plant {
                SysInfraType::TickerPlant => match_ticker_plant_id(template_id, message, process_client.clone()).await,
                SysInfraType::OrderPlant => match_order_plant_id(template_id, message, process_client.clone()).await,
                SysInfraType::HistoryPlant => match_history_plant_id(template_id, message, process_client.clone()).await,
                SysInfraType::PnlPlant => match_pnl_plant_id(template_id, message, process_client.clone()).await,
                SysInfraType::RepositoryPlant => match_repo_plant_id(template_id, message, process_client.clone()).await,
            }
        }
    });

    tokio::spawn(async move {
        'main_loop: loop {
            tokio::select! {
                Some(Ok(message)) = reader.next() => {
                    match message {
                        Message::Binary(bytes) => {
                            // Fast path: direct slice access
                            if bytes.len() >= 4 {
                                length_buf.copy_from_slice(&bytes[..4]);
                                let length = u32::from_be_bytes(length_buf) as usize;

                                // Validate message length
                                if bytes.len() != length + 4 {
                                    eprintln!("Invalid message length. Expected: {}, Got: {}", length + 4, bytes.len());
                                    continue;
                                }

                                // Extract template ID and send message
                                if let Some(template_id) = extract_template_id(&bytes[4..]) {
                                    // Zero-copy slice of the message data
                                    let message_data = bytes[4..].to_vec();

                                    if let Err(e) = tx.send((template_id, message_data)).await {
                                        eprintln!("Fatal error sending message to processor: {}", e);
                                        break 'main_loop;
                                    }
                                }
                            } else {
                                eprintln!("Message too short: {} bytes", bytes.len());
                            }
                        }
                        Message::Close(close_frame) => {
                            println!("Received close message: {:?}. Attempting reconnection.", close_frame);
                                task::spawn(async move {
                                    attempt_reconnect(&client, plant.clone()).await
                                });
                            break 'main_loop;
                        }
                        Message::Frame(frame) => {
                            if format!("{:?}", frame).contains("CloseFrame") {
                                println!("Received close frame. Attempting reconnection.");
                                task::spawn(async move {
                                    attempt_reconnect(&client, plant.clone()).await
                                });
                                break 'main_loop;
                            }
                        }
                        Message::Text(text) => println!("{}", text),
                        Message::Ping(_) | Message::Pong(_) => {}
                    }
                },
                _ = shutdown_receiver.recv() => {
                    println!("Shutdown signal received. Stopping Rithmic response handler.");
                    if let Some((_, writer)) = client.writers.remove(&plant) {
                        if let Err(e) = shutdown_plant(writer).await {
                            eprintln!("Error shutting down plant: {:?}", e);
                        }
                    }
                    break;
                }
            }
        }

        // Cleanup
        println!("Cleaning up Rithmic response handler for plant: {:?}", plant);
        drop(tx);
    });
}

pub async fn shutdown_plant(
    write_stream: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
) -> Result<(), RithmicApiError> {
    let mut write_stream = write_stream.lock().await;
    //Logout Request 12
    let logout_request = RequestLogout {
        template_id: 12,
        user_msg: vec![],
    };

    let mut buf = Vec::new();
    match logout_request.encode(&mut buf) {
        Ok(_) => {}
        Err(e) => return Err(RithmicApiError::ServerErrorDebug(format!("Failed to encode RithmicMessage: {}", e)))
    }

    let length = buf.len() as u32;
    let mut prefixed_msg = length.to_be_bytes().to_vec();
    prefixed_msg.extend(buf);

    match write_stream.send(Message::Binary(prefixed_msg)).await {
        Ok(_) => Ok(()),
        Err(e) => Err(RithmicApiError::Disconnected(e.to_string()))
    }
}

pub async fn send_updates(event: DataServerResponse) {
    for stream_name in RESPONSE_SENDERS.iter() {
        match stream_name.value().send(event.clone()).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e);

            }
        }
    }
}