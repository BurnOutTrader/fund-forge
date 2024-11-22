use std::io::Cursor;
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
use bytes::{Buf, BytesMut};

pub fn handle_rithmic_responses(
    client: Arc<RithmicBrokerageClient>,
    mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    plant: SysInfraType,
) {
    let mut shutdown_receiver = subscribe_server_shutdown();

    // Pre-allocate buffers
    let mut length_buf = [0u8; 4];
    let mut message_buf = BytesMut::with_capacity(16384);

    // Use bounded channel with backpressure
    let (tx, mut rx) = mpsc::channel(1024);

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
                            let mut cursor = Cursor::new(bytes);

                            match cursor.chunk().len() {
                                n if n >= 4 => {
                                    length_buf.copy_from_slice(&cursor.chunk()[..4]);
                                    cursor.advance(4);
                                    let length = u32::from_be_bytes(length_buf) as usize;

                                    if message_buf.capacity() < length {
                                        message_buf.reserve(length - message_buf.capacity());
                                    }
                                    message_buf.resize(length, 0);

                                    if cursor.chunk().len() >= length {
                                        message_buf[..length].copy_from_slice(&cursor.chunk()[..length]);

                                        if let Some(template_id) = extract_template_id(&message_buf) {
                                            let message_data = message_buf[..length].to_vec();

                                            // Wait for the message to be sent - never drop messages
                                            if let Err(e) = tx.send((template_id, message_data)).await {
                                                eprintln!("Fatal error sending message to processor: {}", e);
                                                // In a real system, you might want to initiate shutdown here
                                                break 'main_loop;
                                            }
                                        }
                                    }
                                }
                                _ => eprintln!("Message too short"),
                            }
                        }
                        Message::Close(close_frame) => {
                            println!("Received close message: {:?}. Attempting reconnection.", close_frame);
                            while let None = attempt_reconnect(&client, plant.clone()).await {
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                            continue 'main_loop;
                        }
                        Message::Frame(frame) => {
                            if format!("{:?}", frame).contains("CloseFrame") {
                                println!("Received close frame. Attempting reconnection.");
                                while let None = attempt_reconnect(&client, plant.clone()).await {
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                }
                                continue 'main_loop;
                            }
                        }
                        Message::Text(text) => println!("{}", text),
                        Message::Ping(_) | Message::Pong(_) => {} // Still ignore ping/pong as they don't carry market data
                    }
                },
                _ = shutdown_receiver.recv() => {
                    println!("Shutdown signal received. Stopping Rithmic response handler.");
                    break;
                }
            }
        }

        // Cleanup
        println!("Cleaning up Rithmic response handler for plant: {:?}", plant);
        drop(tx); // Drop sender to close the channel and let processor task complete

        if let Some((_, writer)) = client.writers.remove(&plant) {
            if let Err(e) = shutdown_plant(writer).await {
                eprintln!("Error shutting down plant: {:?}", e);
            }
        }
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