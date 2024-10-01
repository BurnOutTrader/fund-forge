use std::io::Cursor;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use async_std::stream::StreamExt;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::request_account_list::UserType;
use ff_rithmic_api::rithmic_proto_objects::rti::request_market_data_update::UpdateBits;
use ff_rithmic_api::rithmic_proto_objects::rti::request_tick_bar_update::Request;
use ff_rithmic_api::rithmic_proto_objects::rti::response_login_info::UserType::Fcm;
use futures::stream::SplitStream;
use prost::{Message as ProstMessage};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::{client, Message};
use crate::apis::brokerage::broker_enum::Brokerage;
use crate::apis::brokerage::broker_enum::Brokerage::Rithmic;
use crate::apis::rithmic_api::api_client::RithmicClient;
use crate::standardized_types::accounts::ledgers::{AccountInfo, Currency, Ledger};
use crate::standardized_types::enums::Exchange;

/// This Test will fail when the market is closed.
#[tokio::test]
async fn test_rithmic_connection() -> Result<(), Box<dyn std::error::Error>> {
    // Define the file path for credentials
    let servers_file_path = String::from("/Users/kevmonaghan/RustroverProjects/fund-forge/ff_data_server/data/rithmic_credentials/servers.toml".to_string());
    let credentials = String::from("/Users/kevmonaghan/RustroverProjects/fund-forge/ff_data_server/data/rithmic_credentials/topstep_trader.toml".to_string());
    // Define credentials
    let credentials = RithmicCredentials::load_credentials_from_file(&credentials).unwrap();
    let app_name = "fufo:fund-forge".to_string();
    let app_version = "1.0".to_string();

    let rithmic_system = credentials.system_name.clone();
    let brokerage = Brokerage::Rithmic(rithmic_system.clone());
    let ff_rithmic_client = RithmicClient::new(rithmic_system, app_name.clone(), app_version, false, servers_file_path).await;
    let rithmic_client_arc = Arc::new(ff_rithmic_client);

    // send a heartbeat request as a test message, 'RequestHeartbeat' Template number 18
    let heart_beat = RequestHeartbeat {
        template_id: 18,
        user_msg: vec![format!("{} Testing heartbeat", app_name)],
        ssboe: None,
        usecs: None,
    };

    /*    //order plant
        let req = RequestProductRmsInfo {
            template_id: 306,
            user_msg: vec!["callback_id".to_string()],
            fcm_id: None,
            ib_id: Some("NQ".to_string()),
            account_id: Some("S1Sep246906077".to_string()),
        };*/

    //tick plant
    /*    let req = RequestProductCodes {
            template_id: 111 ,
            user_msg: vec![],
            exchange: Some(Exchange::CME.to_string()),
            give_toi_products_only: Some(true),
        };*/

    /*    let req = RequestLoginInfo {
            template_id: 300 ,
            user_msg: vec![],
        };*/

    /*  let req = RequestMarketDataUpdate {
          template_id: 100,
          user_msg: vec![],
          symbol: Some("NQ".to_string()),
          exchange: Some(Exchange::CME.to_string()),
          request: Some(Request::Subscribe.into()),
          update_bits: Some(2),
      };*/
    /*    let req = RequestAccountRmsInfo {
            template_id: 304,
            user_msg: vec![],
            fcm_id: None,
            ib_id: None,
            user_type: Some(UserType::Trader.into()),
        };

        let req = RequestReferenceData {
            template_id: 0,
            user_msg: vec![],
            symbol: None,
            exchange: None,
        }*/

    //history plant
    /*   let req = RequestTimeBarUpdate {
            template_id: 200,
            user_msg: vec![],
            symbol: Some("NQ".to_string()),
            exchange: Some(Exchange::CME.to_string()),
            request: Some(Request::Subscribe.into()),
            bar_type: Some(1),
            bar_type_period: Some(5),
        };*/

    let req = RequestPnLPositionSnapshot {
        template_id: 402 ,
        user_msg: vec![],
        fcm_id: Some("TopstepTrader".to_string()),
        ib_id: Some("TopstepTrader".to_string()),
        account_id: Some("S1Sep246906077".to_string()),
    };


    handle_received_responses(rithmic_client_arc.clone()).await;

    // We can send messages with only a reference to the client, so we can wrap our client in Arc or share it between threads and still utilise all associated functions.
    match rithmic_client_arc.client.send_message(SysInfraType::PnlPlant, req).await {
        Ok(_) => println!("Heart beat sent"),
        Err(e) => eprintln!("Heartbeat send failed: {}", e)
    }

    sleep(Duration::from_secs(120));
    // Logout and Shutdown all connections
    rithmic_client_arc.client.shutdown_all().await.unwrap();
}


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
#[allow(dead_code)]
pub async fn handle_responses_from_order_plant(
    client: Arc<RithmicClient>,
) -> Result<(), RithmicApiError> {
    tokio::task::spawn(async move {
        const PLANT: SysInfraType = SysInfraType::OrderPlant;
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
                                    //println!("Extracted template_id: {}", template_id);
                                    // Now you can use the template_id to determine which type to decode into the concrete types
                                    match template_id {
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
                                        301 => {
                                            if let Ok(msg) = ResponseLoginInfo::decode(&message_buf[..]) {
                                                // Account List Response
                                                // From Server
                                                println!("Response Login Info (Template ID: 303) from Server: {:?}", msg);
                                            }
                                        }
                                        303 => {
                                            if let Ok(msg) = ResponseAccountList::decode(&message_buf[..]) {
                                                // Account List Response
                                                // From Server
                                                println!("Account List Response (Template ID: 303) from Server: {:?}", msg);
                                            }
                                        },
                                        305 => {
                                            if let Ok(msg) = ResponseAccountRmsInfo::decode(&message_buf[..]) {
                                                //println!("Response Account Rms Info (Template ID: 305) from Server: {:?}", msg);
                                                if let Some(id) = &msg.account_id {
                                                    let mut account_info = AccountInfo {
                                                        account_id: id.to_string(),
                                                        brokerage: client.brokerage.clone(),
                                                        cash_value: Default::default(),
                                                        cash_available: Default::default(),
                                                        currency: Currency::USD,
                                                        cash_used: Default::default(),
                                                        positions: vec![],
                                                        is_hedging: false,
                                                        buy_limit: None,
                                                        sell_limit: None,
                                                        max_orders: None,
                                                        daily_max_loss: None,
                                                        daily_max_loss_reset_time: None,
                                                    };
                                                    if let Some(ref currency) = msg.currency {
                                                        account_info.currency = Currency::from_str(&currency);
                                                    }
                                                    if let Some(ref max_size) = msg.buy_limit {
                                                        account_info.buy_limit = Some(Decimal::from_u32(max_size.clone() as u32).unwrap());
                                                    }
                                                    if let Some(ref max_size) = msg.sell_limit {
                                                        account_info.sell_limit = Some(Decimal::from_u32(max_size.clone() as u32).unwrap());
                                                    }
                                                    if let Some(ref max_orders) = msg.max_order_quantity {
                                                        account_info.max_orders = Some(Decimal::from_u32(max_orders.clone() as u32).unwrap());
                                                    }
                                                    if let Some(ref max_loss) = msg.loss_limit {
                                                        account_info.daily_max_loss = Some(Decimal::from_u32(max_loss.clone() as u32).unwrap());
                                                    }
                                                    client.accounts.insert(id.clone(), account_info);
                                                    client.account_rms_info.insert(id.clone(), msg.clone());
                                                    let req = RequestPnLPositionSnapshot {
                                                        template_id: 402,
                                                        user_msg: vec![],
                                                        fcm_id: client.fcm_id.clone(),
                                                        ib_id: client.ib_id.clone(),
                                                        account_id: Some(id.clone()),
                                                    };
                                                    match client.send_message(SysInfraType::PnlPlant, req).await {
                                                        Ok(_) => {}
                                                        Err(_) => {}
                                                    }
                                                }
                                            }
                                        },
                                        307 => {
                                            if let Ok(msg) = ResponseProductRmsInfo::decode(&message_buf[..]) {
                                                // Product RMS Info Response
                                                // From Server
                                                println!("Product RMS Info Response (Template ID: 307) from Server: {:?}", msg);
                                            }
                                        },
                                        309 => {
                                            if let Ok(msg) = ResponseSubscribeForOrderUpdates::decode(&message_buf[..]) {
                                                // Subscribe For Order Updates Response
                                                // From Server
                                                println!("Subscribe For Order Updates Response (Template ID: 309) from Server: {:?}", msg);
                                            }
                                        },
                                        311 => {
                                            if let Ok(msg) = ResponseTradeRoutes::decode(&message_buf[..]) {
                                                // Trade Routes Response
                                                // From Server
                                                println!("Trade Routes Response (Template ID: 311) from Server: {:?}", msg);
                                            }
                                        },
                                        313 => {
                                            if let Ok(msg) = ResponseNewOrder::decode(&message_buf[..]) {
                                                // New Order Response
                                                // From Server
                                                println!("New Order Response (Template ID: 313) from Server: {:?}", msg);
                                            }
                                        },
                                        315 => {
                                            if let Ok(msg) = ResponseModifyOrder::decode(&message_buf[..]) {
                                                // Modify Order Response
                                                // From Server
                                                println!("Modify Order Response (Template ID: 315) from Server: {:?}", msg);
                                            }
                                        },
                                        317 => {
                                            if let Ok(msg) = ResponseCancelOrder::decode(&message_buf[..]) {
                                                // Cancel Order Response
                                                // From Server
                                                println!("Cancel Order Response (Template ID: 317) from Server: {:?}", msg);
                                            }
                                        },
                                        319 => {
                                            if let Ok(msg) = ResponseShowOrderHistoryDates::decode(&message_buf[..]) {
                                                // Show Order History Dates Response
                                                // From Server
                                                println!("Show Order History Dates Response (Template ID: 319) from Server: {:?}", msg);
                                            }
                                        },
                                        321 => {
                                            if let Ok(msg) = ResponseShowOrders::decode(&message_buf[..]) {
                                                // Show Orders Response
                                                // From Server
                                                println!("Show Orders Response (Template ID: 321) from Server: {:?}", msg);
                                            }
                                        },
                                        323 => {
                                            if let Ok(msg) = ResponseShowOrderHistory::decode(&message_buf[..]) {
                                                // Show Order History Response
                                                // From Server
                                                println!("Show Order History Response (Template ID: 323) from Server: {:?}", msg);
                                            }
                                        },
                                        325 => {
                                            if let Ok(msg) = ResponseShowOrderHistorySummary::decode(&message_buf[..]) {
                                                // Show Order History Summary Response
                                                // From Server
                                                println!("Show Order History Summary Response (Template ID: 325) from Server: {:?}", msg);
                                            }
                                        },
                                        327 => {
                                            if let Ok(msg) = ResponseShowOrderHistoryDetail::decode(&message_buf[..]) {
                                                // Show Order History Detail Response
                                                // From Server
                                                println!("Show Order History Detail Response (Template ID: 327) from Server: {:?}", msg);
                                            }
                                        },
                                        329 => {
                                            if let Ok(msg) = ResponseOcoOrder::decode(&message_buf[..]) {
                                                // OCO Order Response
                                                // From Server
                                                println!("OCO Order Response (Template ID: 329) from Server: {:?}", msg);
                                            }
                                        },
                                        331 => {
                                            if let Ok(msg) = ResponseBracketOrder::decode(&message_buf[..]) {
                                                // Bracket Order Response
                                                // From Server
                                                println!("Bracket Order Response (Template ID: 331) from Server: {:?}", msg);
                                            }
                                        },
                                        333 => {
                                            if let Ok(msg) = ResponseUpdateTargetBracketLevel::decode(&message_buf[..]) {
                                                // Update Target Bracket Level Response
                                                // From Server
                                                println!("Update Target Bracket Level Response (Template ID: 333) from Server: {:?}", msg);
                                            }
                                        },
                                        335 => {
                                            if let Ok(msg) = ResponseUpdateStopBracketLevel::decode(&message_buf[..]) {
                                                // Update Stop Bracket Level Response
                                                // From Server
                                                println!("Update Stop Bracket Level Response (Template ID: 335) from Server: {:?}", msg);
                                            }
                                        },
                                        337 => {
                                            if let Ok(msg) = ResponseSubscribeToBracketUpdates::decode(&message_buf[..]) {
                                                // Subscribe To Bracket Updates Response
                                                // From Server
                                                println!("Subscribe To Bracket Updates Response (Template ID: 337) from Server: {:?}", msg);
                                            }
                                        },
                                        339 => {
                                            if let Ok(msg) = ResponseShowBrackets::decode(&message_buf[..]) {
                                                // Show Brackets Response
                                                // From Server
                                                println!("Show Brackets Response (Template ID: 339) from Server: {:?}", msg);
                                            }
                                        },
                                        341 => {
                                            if let Ok(msg) = ResponseShowBracketStops::decode(&message_buf[..]) {
                                                // Show Bracket Stops Response
                                                // From Server
                                                println!("Show Bracket Stops Response (Template ID: 341) from Server: {:?}", msg);
                                            }
                                        },
                                        343 => {
                                            if let Ok(msg) = ResponseListExchangePermissions::decode(&message_buf[..]) {
                                                // List Exchange Permissions Response
                                                // From Server
                                                println!("List Exchange Permissions Response (Template ID: 343) from Server: {:?}", msg);
                                            }
                                        },
                                        345 => {
                                            if let Ok(msg) = ResponseLinkOrders::decode(&message_buf[..]) {
                                                // Link Orders Response
                                                // From Server
                                                println!("Link Orders Response (Template ID: 345) from Server: {:?}", msg);
                                            }
                                        },
                                        347 => {
                                            if let Ok(msg) = ResponseCancelAllOrders::decode(&message_buf[..]) {
                                                // Cancel All Orders Response
                                                // From Server
                                                println!("Cancel All Orders Response (Template ID: 347) from Server: {:?}", msg);
                                            }
                                        },
                                        349 => {
                                            if let Ok(msg) = ResponseEasyToBorrowList::decode(&message_buf[..]) {
                                                // Easy To Borrow List Response
                                                // From Server
                                                println!("Easy To Borrow List Response (Template ID: 349) from Server: {:?}", msg);
                                            }
                                        },
                                        350 => {
                                            if let Ok(msg) = TradeRoute::decode(&message_buf[..]) {
                                                // Trade Route
                                                // From Server
                                                println!("Trade Route (Template ID: 350) from Server: {:?}", msg);
                                            }
                                        },
                                        351 => {
                                            if let Ok(msg) = RithmicOrderNotification::decode(&message_buf[..]) {
                                                // Rithmic Order Notification
                                                // From Server
                                                println!("Rithmic Order Notification (Template ID: 351) from Server: {:?}", msg);
                                            }
                                        },
                                        352 => {
                                            if let Ok(msg) = ExchangeOrderNotification::decode(&message_buf[..]) {
                                                // Exchange Order Notification
                                                // From Server
                                                println!("Exchange Order Notification (Template ID: 352) from Server: {:?}", msg);
                                            }
                                        },
                                        353 => {
                                            if let Ok(msg) = BracketUpdates::decode(&message_buf[..]) {
                                                // Bracket Updates
                                                // From Server
                                                println!("Bracket Updates (Template ID: 353) from Server: {:?}", msg);
                                            }
                                        },
                                        354 => {
                                            if let Ok(msg) = AccountListUpdates::decode(&message_buf[..]) {
                                                // Account List Updates
                                                // From Server
                                                println!("Account List Updates (Template ID: 354) from Server: {:?}", msg);
                                            }
                                        },
                                        355 => {
                                            if let Ok(msg) = UpdateEasyToBorrowList::decode(&message_buf[..]) {
                                                // Update Easy To Borrow List
                                                // From Server
                                                println!("Update Easy To Borrow List (Template ID: 355) from Server: {:?}", msg);
                                            }
                                        },
                                        3501 => {
                                            if let Ok(msg) = ResponseModifyOrderReferenceData::decode(&message_buf[..]) {
                                                // Modify Order Reference Data Response
                                                // From Server
                                                println!("Modify Order Reference Data Response (Template ID: 3501) from Server: {:?}", msg);
                                            }
                                        },
                                        3503 => {
                                            if let Ok(msg) = ResponseOrderSessionConfig::decode(&message_buf[..]) {
                                                // Order Session Config Response
                                                // From Server
                                                println!("Order Session Config Response (Template ID: 3503) from Server: {:?}", msg);
                                            }
                                        },
                                        3505 => {
                                            if let Ok(msg) = ResponseExitPosition::decode(&message_buf[..]) {
                                                // Exit Position Response
                                                // From Server
                                                println!("Exit Position Response (Template ID: 3505) from Server: {:?}", msg);
                                            }
                                        },
                                        3507 => {
                                            if let Ok(msg) = ResponseReplayExecutions::decode(&message_buf[..]) {
                                                // Replay Executions Response
                                                // From Server
                                                println!("Replay Executions Response (Template ID: 3507) from Server: {:?}", msg);
                                            }
                                        },
                                        3509 => {
                                            if let Ok(msg) = ResponseAccountRmsUpdates::decode(&message_buf[..]) {
                                                // Account RMS Updates Response
                                                // From Server
                                                println!("Account RMS Updates Response (Template ID: 3509) from Server: {:?}", msg);
                                            }
                                        },
                                        356 => {
                                            if let Ok(msg) = AccountRmsUpdates::decode(&message_buf[..]) {
                                                // Account RMS Updates
                                                // From Server
                                                println!("Account RMS Updates (Template ID: 356) from Server: {:?}", msg);
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

#[allow(dead_code)]
pub async fn handle_responses_from_history_plant(
    client: Arc<RithmicClient>,
) -> Result<(), RithmicApiError> {
    tokio::task::spawn(async move {
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
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_responses_from_pnl_plant(
    client: Arc<RithmicClient>,
) -> Result<(), RithmicApiError> {
    tokio::task::spawn(async move {
        const PLANT: SysInfraType = SysInfraType::PnlPlant;
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
                                        401 => {
                                            if let Ok(msg) = ResponsePnLPositionUpdates::decode(&message_buf[..]) {
                                                // PnL Position Updates Response
                                                // From Server
                                                println!("PnL Position Updates Response (Template ID: 401) from Server: {:?}", msg);
                                            }
                                        },
                                        403 => {
                                            if let Ok(msg) = ResponsePnLPositionSnapshot::decode(&message_buf[..]) {
                                                // PnL Position Snapshot Response
                                                // From Server
                                                println!("PnL Position Snapshot Response (Template ID: 403) from Server: {:?}", msg);

                                            }
                                        },
                                        450 => {
                                            if let Ok(msg) = InstrumentPnLPositionUpdate::decode(&message_buf[..]) {
                                                // Instrument PnL Position Update
                                                // From Server
                                                println!("Instrument PnL Position Update (Template ID: 450) from Server: {:?}", msg);
                                            }
                                        },
                                        451 => {
                                            if let Ok(msg) = AccountPnLPositionUpdate::decode(&message_buf[..]) {
                                                // Account PnL Position Update
                                                // From Server
                                                println!("Account PnL Position Update (Template ID: 451) from Server: {:?}", msg);
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


#[allow(dead_code)]
pub async fn handle_responses_from_repo_plant(
    client: Arc<RithmicClient>,
) -> Result<(), RithmicApiError> {
    tokio::task::spawn(async move {
        const PLANT: SysInfraType = SysInfraType::RepositoryPlant;
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
                                        501 => {
                                            if let Ok(msg) = ResponseListUnacceptedAgreements::decode(&message_buf[..]) {
                                                // List Unaccepted Agreements Response
                                                // From Server
                                                println!("List Unaccepted Agreements Response (Template ID: 501) from Server: {:?}", msg);
                                            }
                                        },
                                        503 => {
                                            if let Ok(msg) = ResponseListAcceptedAgreements::decode(&message_buf[..]) {
                                                // List Accepted Agreements Response
                                                // From Server
                                                println!("List Accepted Agreements Response (Template ID: 503) from Server: {:?}", msg);
                                            }
                                        },
                                        505 => {
                                            if let Ok(msg) = ResponseAcceptAgreement::decode(&message_buf[..]) {
                                                // Accept Agreement Response
                                                // From Server
                                                println!("Accept Agreement Response (Template ID: 505) from Server: {:?}", msg);
                                            }
                                        },
                                        507 => {
                                            if let Ok(msg) = ResponseShowAgreement::decode(&message_buf[..]) {
                                                // Show Agreement Response
                                                // From Server
                                                println!("Show Agreement Response (Template ID: 507) from Server: {:?}", msg);
                                            }
                                        },
                                        509 => {
                                            if let Ok(msg) = ResponseSetRithmicMrktDataSelfCertStatus::decode(&message_buf[..]) {
                                                // Set Rithmic MarketData Self Certification Status Response
                                                // From Server
                                                println!("Set Rithmic MarketData Self Certification Status Response (Template ID: 509) from Server: {:?}", msg);
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
