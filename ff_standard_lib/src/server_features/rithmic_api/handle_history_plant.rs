use std::io::Cursor;
use std::sync::Arc;
use async_std::stream::StreamExt;
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use prost::{Message as ProstMessage};
use tokio::task::JoinHandle;
use tungstenite::{Message};
#[allow(unused_imports)]
use crate::standardized_types::broker_enum::Brokerage;
use crate::server_features::rithmic_api::api_client::RithmicClient;

#[allow(dead_code)]
pub async fn handle_responses_from_history_plant(
    client: Arc<RithmicClient>,
)  -> JoinHandle<()> {
    let handle = tokio::task::spawn(async move {
        const PLANT: SysInfraType = SysInfraType::HistoryPlant;
        let reader = client.readers.get(&PLANT).unwrap().value().clone();
        let mut reader = reader.lock().await;
        while let Some(message) = reader.next().await {
            match message {
                Ok(message) => {
                    // Tungstenite messages, if you use ProstMessage here you will get a trait related compile time error
                    match message {
                        Message::Text(text) => {
                            println!("{}", text)
                        }
                        Message::Binary(bytes) => {
                            // spawn a new task so that we can handle next message faster.
                            let client = client.clone();
                            tokio::task::spawn(async move {
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
                                if let Some(template_id) = client.client.extract_template_id(&message_buf) {
                                    println!("Extracted template_id: {}", template_id);
                                    // Now you can use the template_id to determine which type to decode into the concrete types
                                    match template_id {
                                        11 => {
                                            if let Ok(msg) = ResponseLogin::decode(&message_buf[..]) {
                                                // Login Response
                                                // From Server
                                                println!("Login Response (Template ID: 11) from Server: {:?}", msg);
                                            }
                                        },
                                        13 => {
                                            if let Ok(msg) = ResponseLogout::decode(&message_buf[..]) {
                                                // Logout Response
                                                // From Server
                                                println!("Logout Response (Template ID: 13) from Server: {:?}", msg);
                                            }
                                        },
                                        15 => {
                                            if let Ok(msg) = ResponseReferenceData::decode(&message_buf[..]) {
                                                // Reference Data Response
                                                // From Server
                                                println!("Reference Data Response (Template ID: 15) from Server: {:?}", msg);
                                            }
                                        },
                                        17 => {
                                            if let Ok(msg) = ResponseRithmicSystemInfo::decode(&message_buf[..]) {
                                                // Rithmic System Info Response
                                                // From Server
                                                println!("Rithmic System Info Response (Template ID: 17) from Server: {:?}", msg);
                                            }
                                        },
                                        19 => {
                                            if let Ok(msg) = ResponseHeartbeat::decode(&message_buf[..]) {
                                                // Response Heartbeat
                                                // From Server
                                                println!("Response Heartbeat (Template ID: 19) from Server: {:?}", msg);
                                            }
                                        },
                                        201 => {
                                            if let Ok(msg) = ResponseTimeBarUpdate::decode(&message_buf[..]) {
                                                // Time Bar Update Response
                                                // From Server
                                                println!("Time Bar Update Response (Template ID: 201) from Server: {:?}", msg);
                                            }
                                        },
                                        203 => {
                                            if let Ok(msg) = ResponseTimeBarReplay::decode(&message_buf[..]) {
                                                // Time Bar Replay Response
                                                // From Server
                                                println!("Time Bar Replay Response (Template ID: 203) from Server: {:?}", msg);
                                            }
                                        },
                                        205 => {
                                            if let Ok(msg) = ResponseTickBarUpdate::decode(&message_buf[..]) {
                                                // Tick Bar Update Response
                                                // From Server
                                                println!("Tick Bar Update Response (Template ID: 205) from Server: {:?}", msg);
                                            }
                                        },
                                        207 => {
                                            if let Ok(msg) = ResponseTickBarReplay::decode(&message_buf[..]) {
                                                // Tick Bar Replay Response
                                                // From Server
                                                println!("Tick Bar Replay Response (Template ID: 207) from Server: {:?}", msg);
                                            }
                                        },
                                        208 => {
                                            if let Ok(msg) = RequestVolumeProfileMinuteBars::decode(&message_buf[..]) {
                                                // Volume Profile Minute Bars Request
                                                // From Client
                                                println!("Volume Profile Minute Bars Request (Template ID: 208) from Client: {:?}", msg);
                                            }
                                        },
                                        209 => {
                                            if let Ok(msg) = ResponseVolumeProfileMinuteBars::decode(&message_buf[..]) {
                                                // Volume Profile Minute Bars Response
                                                // From Server
                                                println!("Volume Profile Minute Bars Response (Template ID: 209) from Server: {:?}", msg);
                                            }
                                        },
                                        211 => {
                                            if let Ok(msg) = ResponseResumeBars::decode(&message_buf[..]) {
                                                // Resume Bars Response
                                                // From Server
                                                println!("Resume Bars Response (Template ID: 211) from Server: {:?}", msg);
                                            }
                                        },
                                        250 => {
                                            if let Ok(msg) = TimeBar::decode(&message_buf[..]) {
                                                // Time Bar
                                                // From Server
                                                println!("Time Bar (Template ID: 250) from Server: {:?}", msg);
                                            }
                                        },
                                        251 => {
                                            if let Ok(msg) = TickBar::decode(&message_buf[..]) {
                                                // Tick Bar
                                                // From Server
                                                println!("Tick Bar (Template ID: 251) from Server: {:?}", msg);
                                            }
                                        },
                                        _ => println!("No match for template_id: {}", template_id)
                                    }
                                }
                            });
                        }
                        Message::Ping(ping) => {
                            println!("{:?}", ping)
                        }
                        Message::Pong(pong) => {
                            println!("{:?}", pong)
                        }
                        Message::Close(close) => {
                            // receive this message when market is closed.
                            // received: Ok(Close(Some(CloseFrame { code: Normal, reason: "normal closure" })))
                            println!("{:?}", close)
                        }
                        Message::Frame(frame) => {
                            //This message is sent on weekends, you can use this message to schedule a reconnection attempt for market open.
                            /* Example of received market closed message
                                Some(CloseFrame { code: Normal, reason: "normal closure" })
                                Error: ServerErrorDebug("Failed to send RithmicMessage, possible disconnect, try reconnecting to plant TickerPlant: Trying to work with closed connection")
                            */
                            println!("{}", frame)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("failed to receive message: {}", e)
                }
            }
        }
    });
    handle
}