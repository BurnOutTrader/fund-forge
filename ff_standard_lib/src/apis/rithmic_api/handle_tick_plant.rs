use std::io::Cursor;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use async_std::stream::StreamExt;
use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;
use ff_rithmic_api::errors::RithmicApiError;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestHeartbeat, RequestProductRmsInfo, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::request_account_list::UserType;
use ff_rithmic_api::systems::RithmicSystem;
use futures::stream::SplitStream;
use prost::{Message as ProstMessage};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use crate::apis::brokerage::broker_enum::Brokerage::Rithmic;
use crate::apis::rithmic_api::api_client::RithmicClient;

/// This Test will fail when the market is closed.
#[tokio::test]
async fn test_rithmic_connection() -> Result<(), Box<dyn std::error::Error>> {
    // Define the file path for credentials
   /* let servers_file_path = String::from("/Users/kevmonaghan/RustroverProjects/fund-forge/ff_data_server/data/rithmic_credentials/servers.toml".to_string());
    let credentials = String::from("/Users/kevmonaghan/RustroverProjects/fund-forge/ff_data_server/data/rithmic_credentials/topstep_trader.toml".to_string());
    // Define credentials
    let credentials = RithmicCredentials::load_credentials_from_file(&credentials).unwrap();
    let app_name = "fufo:fund-forge".to_string();
    let app_version = "1.0".to_string();

    let rithmic_system = credentials.system_name.clone();
    let brokerage = Rithmic(rithmic_system);
    let ff_rithmic_client = RithmicClient::new(RithmicSystem::TopstepTrader, app_name.clone(), app_version, false, servers_file_path);
    let rithmic_client_arc = Arc::new(ff_rithmic_client);

    // send a heartbeat request as a test message, 'RequestHeartbeat' Template number 18
    let heart_beat = RequestHeartbeat {
        template_id: 18,
        user_msg: vec![format!("{} Testing heartbeat", app_name)],
        ssboe: None,
        usecs: None,
    };

    let req = RequestProductRmsInfo {
        template_id: 306,
        user_msg: vec!["callback_id".to_string()],
        fcm_id: None,
        ib_id: None,
        account_id: None,
    };

    let accounts = RequestAccountList {
        template_id: 302,
        user_msg: vec![],
        fcm_id: None,
        ib_id: None,
        user_type: Some(UserType::Trader.into())
    };


    // We can send messages with only a reference to the client, so we can wrap our client in Arc or share it between threads and still utilise all associated functions.
    match rithmic_client_arc.client.send_message(SysInfraType::OrderPlant, accounts).await {
        Ok(_) => println!("Heart beat sent"),
        Err(e) => eprintln!("Heartbeat send failed: {}", e)
    }

    sleep(Duration::from_secs(10));
    // Logout and Shutdown all connections
    rithmic_client_arc.client.shutdown_all().await?;*/

    Ok(())
}
