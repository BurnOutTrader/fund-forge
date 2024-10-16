use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[allow(unused_imports)]
use std::time::Duration;
use async_std::stream::StreamExt;
use ff_rithmic_api::api_client::extract_template_id;
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::RequestLogout;
use futures::SinkExt;
use prost::Message as ProstMessage;
#[allow(unused_imports)]
use tokio::time::sleep;
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use crate::rithmic_api::api_client::RithmicClient;
use futures::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use ff_standard_lib::messages::data_server_messaging::DataServerResponse;
use crate::request_handlers::RESPONSE_SENDERS;
use crate::rithmic_api::plant_handlers::handle_history_plant::match_history_plant_id;
use crate::rithmic_api::plant_handlers::handle_order_plant::match_order_plant_id;
use crate::rithmic_api::plant_handlers::handle_pnl_plant::match_pnl_plant_id;
use crate::rithmic_api::plant_handlers::handle_repo_plant::match_repo_plant_id;
use crate::rithmic_api::plant_handlers::handle_tick_plant::match_ticker_plant_id;
use crate::rithmic_api::plant_handlers::reconnect::attempt_reconnect;

pub fn handle_rithmic_responses(
    client: Arc<RithmicClient>,
    mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    plant: SysInfraType,
    running: Arc<AtomicBool>
) {
    // Task 2: Handle Rithmic responses
    tokio::spawn(async move {
        while running.load(Ordering::SeqCst) {
            match reader.next().await {
                Some(Ok(message)) => {
                    match message {
                        Message::Binary(bytes) => {
                            let client = client.clone();
                            let mut cursor = Cursor::new(bytes);
                            let mut length_buf = [0u8; 4];
                            if let Err(e) = tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut length_buf).await {
                                eprintln!("Failed to read length: {}", e);
                                continue;
                            }
                            let length = u32::from_be_bytes(length_buf) as usize;

                            let mut message_buf = vec![0u8; length];
                            if let Err(e) = tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut message_buf).await {
                                eprintln!("Failed to read message: {}", e);
                                continue;
                            }

                            tokio::spawn(async move {
                                if let Some(template_id) = extract_template_id(&message_buf) {
                                    match plant {
                                        SysInfraType::TickerPlant => match_ticker_plant_id(template_id, message_buf, client.clone()).await,
                                        SysInfraType::OrderPlant => match_order_plant_id(template_id, message_buf, client.clone()).await,
                                        SysInfraType::HistoryPlant => match_history_plant_id(template_id, message_buf, client.clone()).await,
                                        SysInfraType::PnlPlant => match_pnl_plant_id(template_id, message_buf, client.clone()).await,
                                        SysInfraType::RepositoryPlant => match_repo_plant_id(template_id, message_buf, client.clone()).await,
                                    }
                                }
                            });
                        }
                        Message::Text(text) => println!("{}", text),
                        Message::Ping(ping) => println!("{:?}", ping),
                        Message::Pong(pong) => println!("{:?}", pong),
                        Message::Close(close_frame) => {
                            println!("Received close message: {:?}", close_frame);
                            if let Some(new_reader) = attempt_reconnect(&client, plant.clone()).await {
                                reader = new_reader;
                            } else {
                                println!("Failed to reconnect after close message. Exiting.");
                                break;
                            }
                        }
                        Message::Frame(frame) => {
                            let frame_str = format!("{:?}", frame);
                            if frame_str.contains("CloseFrame") {
                                println!("Received close frame. Attempting reconnection.");
                                if let Some(new_reader) = attempt_reconnect(&client, plant.clone()).await {
                                    reader = new_reader;
                                } else {
                                    println!("Failed to reconnect. Exiting.");
                                    break;
                                }
                            } else {
                                println!("Received frame: {:?}", frame);
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    eprintln!("WebSocket error: {:?}", e);
                    if let Some(new_reader) = attempt_reconnect(&client, plant.clone()).await {
                        reader = new_reader;
                    } else {
                        break;
                    }
                }
                None => {
                    println!("WebSocket stream ended");
                    if let Some(new_reader) = attempt_reconnect(&client, plant.clone()).await {
                        reader = new_reader;
                    } else {
                        break;
                    }
                }
            }
        }

        println!("Shutting down Rithmic response handler for plant: {:?}", plant);
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
            Err(e) => eprintln!("failed to forward ResponseNewOrder 313 to strategy stream {}", e)
        }
    }
}