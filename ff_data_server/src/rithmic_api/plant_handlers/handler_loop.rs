use std::io::Cursor;
use std::sync::Arc;
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
use futures::FutureExt;
use futures::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use crate::rithmic_api::plant_handlers::handle_history_plant::match_history_plant_id;
use crate::rithmic_api::plant_handlers::handle_order_plant::match_order_plant_id;
use crate::rithmic_api::plant_handlers::handle_pnl_plant::match_pnl_plant_id;
use crate::rithmic_api::plant_handlers::handle_repo_plant::match_repo_plant_id;
use crate::rithmic_api::plant_handlers::handle_tick_plant::match_ticker_plant_id;
use crate::rithmic_api::plant_handlers::reconnect::attempt_reconnect;
use crate::subscribe_server_shutdown;

/// we use extract_template_id() to get the template id using the field_number 154467 without casting to any concrete type, then we map to the concrete type and handle that message.
pub async fn handle_rithmic_responses(
    client: Arc<RithmicClient>,
    reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    plant: SysInfraType
) {
        tokio::task::spawn(async move {
            let mut reader = reader;
            let mut shutdown_receiver = subscribe_server_shutdown();
            'main_loop: loop {
                tokio::select! {
                 _ = shutdown_receiver.recv() => {
                    if let Some((_, writer)) = client.writers.remove(&plant) {
                         match shutdown_plant(writer).await {
                            Ok(_) => {},
                            Err(_) => {}
                        }
                    }
                    break 'main_loop;
                },
                message = reader.next().fuse() => {
                    match message {
                        Some(Ok(message)) => {
                            match message {
                                // Tungstenite messages, if you use ProstMessage here you will get a trait related compile time error
                                Message::Binary(bytes) => {
                                    let client = client.clone();
                                    // spawn a new task so that we can handle next message faster.
                                    //messages will be forwarded here
                                    let mut cursor = Cursor::new(bytes);
                                    // Read the 4-byte length header
                                    let mut length_buf = [0u8; 4];
                                    let _ = tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut length_buf).await.map_err(RithmicApiError::Io);
                                    let length = u32::from_be_bytes(length_buf) as usize;
                                    //println!("Length: {}", length);

                                    // Read the Protobuf message
                                    let mut message_buf = vec![0u8; length];

                                    match tokio::io::AsyncReadExt::read_exact(&mut cursor, &mut message_buf).await.map_err(RithmicApiError::Io) {
                                        Ok(_) => {}
                                        Err(e) => eprintln!("Failed to read_extract message: {}", e)
                                    }
                                    tokio::task::spawn(async move {
                                        if let Some(template_id) = extract_template_id(&message_buf) {
                                            //println!("Extracted template_id: {}", template_id);
                                            match plant {
                                                SysInfraType::TickerPlant =>  match_ticker_plant_id(template_id, message_buf, client.clone()).await,
                                                SysInfraType::OrderPlant =>  match_order_plant_id(template_id, message_buf, client.clone()).await,
                                                SysInfraType::HistoryPlant =>  match_history_plant_id(template_id, message_buf, client.clone()).await,
                                                SysInfraType::PnlPlant =>  match_pnl_plant_id(template_id, message_buf, client.clone()).await,
                                                SysInfraType::RepositoryPlant =>  match_repo_plant_id(template_id, message_buf, client.clone()).await,
                                            }
                                        }
                                    });
                                }
                                Message::Text(text) => {
                                    println!("{}", text)
                                }
                                Message::Ping(ping) => {
                                    println!("{:?}", ping)
                                }
                                Message::Pong(pong) => {
                                    println!("{:?}", pong)
                                }
                                Message::Close(close_frame) => {
                                    if let Some(frame) = close_frame {
                                        println!("Received close frame: code = {:?}, reason = {}", frame.code, frame.reason);
                                        if let Some(new_reader) = attempt_reconnect(&client, plant.clone()).await {
                                            reader = new_reader;
                                            continue; // Skip to the next iteration of the main loop
                                        } else {
                                            println!("Failed to reconnect after normal closure. Exiting.");
                                            break;
                                        }
                                    } else {
                                        println!("Received close message without a frame.");
                                    }
                                    // Attempt reconnection for any close message
                                    if let Some(new_reader) = attempt_reconnect(&client, plant.clone()).await {
                                        reader = new_reader;
                                    } else {
                                        println!("Failed to reconnect after close message. Exiting.");
                                        break;
                                    }
                                }
                                Message::Frame(frame) => {
                                          /* Example of received market closed message
                                        Some(CloseFrame { code: Normal, reason: "normal closure" })
                                        Error: ServerErrorDebug("Failed to send RithmicMessage, possible disconnect, try reconnecting to plant TickerPlant: Trying to work with closed connection")
                                    */
                                        let frame_str = format!("{:?}", frame);
                                    if frame_str.contains("CloseFrame") {
                                        println!("Received close frame with normal closure. Attempting reconnection.");
                                        if let Some(new_reader) = attempt_reconnect(&client, plant.clone()).await {
                                            reader = new_reader;
                                            continue;
                                        } else {
                                            println!("Failed to reconnect. Exiting.");
                                            break;
                                        }
                                    } else {
                                        println!("Received frame: {:?}", frame);
                                        // Process other types of frames here
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
            }
        }
            eprintln!("Tick Plant dropped");
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