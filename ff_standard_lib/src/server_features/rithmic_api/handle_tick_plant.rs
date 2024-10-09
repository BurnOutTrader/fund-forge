use std::io::Cursor;
use std::sync::Arc;
#[allow(unused_imports)]
use std::time::Duration;
use async_std::stream::StreamExt;
use chrono::Utc;
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use prost::{Message as ProstMessage};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
#[allow(unused_imports)]
use tokio::time::sleep;
use tungstenite::{Message};
use crate::messages::data_server_messaging::DataServerResponse;
#[allow(unused_imports)]
use crate::standardized_types::broker_enum::Brokerage;
use crate::server_features::rithmic_api::api_client::RithmicClient;
use crate::server_features::rithmic_api::handle_history_plant::handle_responses_from_history_plant;
use crate::server_features::rithmic_api::handle_order_plant::handle_responses_from_order_plant;
use crate::server_features::rithmic_api::handle_pnl_plant::handle_responses_from_pnl_plant;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::tick::Tick;
use crate::standardized_types::enums::{FuturesExchange, MarketType, OrderSide};
use crate::standardized_types::subscriptions::Symbol;

pub async fn handle_received_responses(
    client: Arc<RithmicClient>,
) {
    handle_responses_from_ticker_plant(client.clone()).await.unwrap();
    handle_responses_from_order_plant(client.clone()).await.unwrap();
    handle_responses_from_history_plant(client.clone()).await.unwrap();
    handle_responses_from_pnl_plant(client.clone()).await.unwrap();
    //handle_responses_from_repo_plant(client).await.unwrap();
}

#[allow(dead_code)]
/// we use extract_template_id() to get the template id using the field_number 154467 without casting to any concrete type, then we map to the concrete type and handle that message.
pub async fn handle_responses_from_ticker_plant(
    client: Arc<RithmicClient>,
) -> Result<(), RithmicApiError> {
    const PLANT: SysInfraType = SysInfraType::TickerPlant;
    tokio::task::spawn(async move {
        let reader = client.readers.get(&PLANT).unwrap().value().clone();
        let mut reader = reader.lock().await;
        while let Some(message) = reader.next().await {
            match message {
                Ok(message) => {
                    match message {
                        // Tungstenite messages, if you use ProstMessage here you will get a trait related compile time error
                        Message::Text(text) => {
                            println!("{}", text)
                        }
                        Message::Binary(bytes) => {
                            let client = client.clone();
                            tokio::task::spawn(async move {
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
                                        101 => {
                                            if let Ok(msg) = ResponseMarketDataUpdate::decode(&message_buf[..]) {
                                                // Market Data Update Response
                                                // From Server
                                                println!("Market Data Update Response (Template ID: 101) from Server: {:?}", msg);
                                            }
                                        },
                                        103 => {
                                            if let Ok(msg) = ResponseGetInstrumentByUnderlying::decode(&message_buf[..]) {
                                                // Get Instrument by Underlying Response
                                                // From Server
                                                println!("Get Instrument by Underlying Response (Template ID: 103) from Server: {:?}", msg);
                                            }
                                        },
                                        104 => {
                                            if let Ok(msg) = ResponseGetInstrumentByUnderlyingKeys::decode(&message_buf[..]) {
                                                // Get Instrument by Underlying Keys Response
                                                // From Server
                                                println!("Get Instrument by Underlying Keys Response (Template ID: 104) from Server: {:?}", msg);
                                            }
                                        },
                                        106 => {
                                            if let Ok(msg) = ResponseMarketDataUpdateByUnderlying::decode(&message_buf[..]) {
                                                // Market Data Update by Underlying Response
                                                // From Server
                                                println!("Market Data Update by Underlying Response (Template ID: 106) from Server: {:?}", msg);
                                            }
                                        },
                                        108 => {
                                            if let Ok(msg) = ResponseGiveTickSizeTypeTable::decode(&message_buf[..]) {
                                                // Give Tick Size Type Table Response
                                                // From Server
                                                println!("Give Tick Size Type Table Response (Template ID: 108) from Server: {:?}", msg);
                                            }
                                        },
                                        110 => {
                                            if let Ok(msg) = ResponseSearchSymbols::decode(&message_buf[..]) {
                                                // Search Symbols Response
                                                // From Server
                                                println!("Search Symbols Response (Template ID: 110) from Server: {:?}", msg);
                                            }
                                        },
                                        112 => {
                                            if let Ok(msg) = ResponseProductCodes::decode(&message_buf[..]) {
                                                // Product Codes Response
                                                // From Server
                                                println!("Product Codes Response (Template ID: 112) from Server: {:?}", msg);
                                            }
                                        },
                                        114 => {
                                            if let Ok(msg) = ResponseFrontMonthContract::decode(&message_buf[..]) {
                                                // Front Month Contract Response
                                                // From Server
                                                println!("Front Month Contract Response (Template ID: 114) from Server: {:?}", msg);
                                            }
                                        },
                                        116 => {
                                            if let Ok(msg) = ResponseDepthByOrderSnapshot::decode(&message_buf[..]) {
                                                // Depth By Order Snapshot Response
                                                // From Server
                                                println!("Depth By Order Snapshot Response (Template ID: 116) from Server: {:?}", msg);
                                            }
                                        },
                                        118 => {
                                            if let Ok(msg) = ResponseDepthByOrderUpdates::decode(&message_buf[..]) {
                                                // Depth By Order Updates Response
                                                // From Server
                                                println!("Depth By Order Updates Response (Template ID: 118) from Server: {:?}", msg);
                                            }
                                        },
                                        120 => {
                                            if let Ok(msg) = ResponseGetVolumeAtPrice::decode(&message_buf[..]) {
                                                // Get Volume At Price Response
                                                // From Server
                                                println!("Get Volume At Price Response (Template ID: 120) from Server: {:?}", msg);
                                            }
                                        },
                                        122 => {
                                            if let Ok(msg) = ResponseAuxilliaryReferenceData::decode(&message_buf[..]) {
                                                // Auxiliary Reference Data Response
                                                // From Server
                                                println!("Auxiliary Reference Data Response (Template ID: 122) from Server: {:?}", msg);
                                            }
                                        },
                                        150 => {
                                            if let Ok(msg) = LastTrade::decode(&message_buf[..]) {
                                                // Last Trade
                                                // From Server
                                                println!("Last Trade (Template ID: 150) from Server: {:?}", msg);
                                                let exchange = match msg.exchange {
                                                    None => return,
                                                    Some(exchange) => {
                                                        match FuturesExchange::from_string(&exchange) {
                                                            Ok(ex) => ex,
                                                            Err(_) => return
                                                        }
                                                    }
                                                };
                                                let volume = match msg.trade_size {
                                                    None => return,
                                                    Some(size) => Decimal::from_i32(size).unwrap()
                                                };
                                                let price = match msg.trade_price {
                                                    None => return,
                                                    Some(price) => Decimal::from_f64(price).unwrap()
                                                };
                                                let side = match msg.aggressor {
                                                    None => return,
                                                    Some(aggressor) => {
                                                        match aggressor {
                                                            1 => OrderSide::Buy,
                                                            2 => OrderSide::Sell,
                                                            _ => return,
                                                        }
                                                    }
                                                };
                                                let symbol = Symbol::new(msg.symbol.unwrap(), client.data_vendor.clone(), MarketType::Futures(exchange));
                                                let tick = Tick::new(symbol, price, Utc::now().to_string(), volume, side);
                                                if let Some(broadcaster) = client.tick_feed_broadcasters.get(&tick.symbol.name) {
                                                    broadcaster.broadcast(DataServerResponse::BaseDataUpdates(BaseDataEnum::Tick(tick))).await;
                                                }
                                            }
                                        },
                                        151 => {
                                            if let Ok(msg) = BestBidOffer::decode(&message_buf[..]) {
                                                // Best Bid Offer
                                                // From Server
                                                println!("Best Bid Offer (Template ID: 151) from Server: {:?}", msg);
                                            }
                                        },
                                        152 => {
                                            if let Ok(msg) = TradeStatistics::decode(&message_buf[..]) {
                                                // Trade Statistics
                                                // From Server
                                                println!("Trade Statistics (Template ID: 152) from Server: {:?}", msg);
                                            }
                                        },
                                        153 => {
                                            if let Ok(msg) = QuoteStatistics::decode(&message_buf[..]) {
                                                // Quote Statistics
                                                // From Server
                                                println!("Quote Statistics (Template ID: 153) from Server: {:?}", msg);
                                            }
                                        },
                                        154 => {
                                            if let Ok(msg) = IndicatorPrices::decode(&message_buf[..]) {
                                                // Indicator Prices
                                                // From Server
                                                println!("Indicator Prices (Template ID: 154) from Server: {:?}", msg);
                                            }
                                        },
                                        155 => {
                                            if let Ok(msg) = EndOfDayPrices::decode(&message_buf[..]) {
                                                // End Of Day Prices
                                                // From Server
                                                println!("End Of Day Prices (Template ID: 155) from Server: {:?}", msg);
                                            }
                                        },
                                        156 => {
                                            if let Ok(msg) = OrderBook::decode(&message_buf[..]) {
                                                // Order Book
                                                // From Server
                                                println!("Order Book (Template ID: 156) from Server: {:?}", msg);
                                            }
                                        },
                                        157 => {
                                            if let Ok(msg) = MarketMode::decode(&message_buf[..]) {
                                                // Market Mode
                                                // From Server
                                                println!("Market Mode (Template ID: 157) from Server: {:?}", msg);
                                            }
                                        },
                                        158 => {
                                            if let Ok(msg) = OpenInterest::decode(&message_buf[..]) {
                                                // Open Interest
                                                // From Server
                                                println!("Open Interest (Template ID: 158) from Server: {:?}", msg);
                                            }
                                        },
                                        159 => {
                                            if let Ok(msg) = FrontMonthContractUpdate::decode(&message_buf[..]) {
                                                // Front Month Contract Update
                                                // From Server
                                                println!("Front Month Contract Update (Template ID: 159) from Server: {:?}", msg);
                                            }
                                        },
                                        160 => {
                                            if let Ok(msg) = DepthByOrder::decode(&message_buf[..]) {
                                                // Depth By Order
                                                // From Server
                                                println!("Depth By Order (Template ID: 160) from Server: {:?}", msg);
                                            }
                                        },
                                        161 => {
                                            if let Ok(msg) = DepthByOrderEndEvent::decode(&message_buf[..]) {
                                                // Depth By Order End Event
                                                // From Server
                                                println!("DepthByOrderEndEvent (Template ID: 161) from Server: {:?}", msg);
                                            }
                                        },
                                        162 => {
                                            if let Ok(msg) = SymbolMarginRate::decode(&message_buf[..]) {
                                                // Symbol Margin Rate
                                                // From Server
                                                println!("Symbol Margin Rate (Template ID: 162) from Server: {:?}", msg);
                                            }
                                        },
                                        163 => {
                                            if let Ok(msg) = OrderPriceLimits::decode(&message_buf[..]) {
                                                // Order Price Limits
                                                // From Server
                                                println!("Order Price Limits (Template ID: 163) from Server: {:?}", msg);
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
    Ok(())
}