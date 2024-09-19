use ff_standard_lib::apis::rithmic_api::handle_tick_plant::handle_received_responses;

use std::sync::Arc;

use ff_rithmic_api::api_client::RithmicApiClient;
use ff_rithmic_api::credentials::RithmicCredentials;

use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestGiveTickSizeTypeTable, RequestHeartbeat, RequestProductRmsInfo, RequestRithmicSystemGatewayInfo, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};

use tokio::task;

#[tokio::main]
async fn main() {
    // Define the file path for credentials
    let file_path = String::from("/Users/kevmonaghan/RustroverProjects/fund-forge/ff_data_server/rithmic_credentials/rithmic_credentials_test.toml".to_string());

    // Define credentials
    let credentials = RithmicCredentials::load_credentials_from_file(&file_path).unwrap();
    let app_name = credentials.app_name.clone();
    // Save credentials to file
    //credentials.save_credentials_to_file(&file_path)?;

    // Create a new RithmicApiClient instance
    let rithmic_api = RithmicApiClient::new(credentials);
    let rithmic_api_arc = Arc::new(rithmic_api);

    let rithmic_api_clone = rithmic_api_arc.clone();
/*    task::spawn(async move {
        let ticker_receiver = rithmic_api_clone.connect_and_login(SysInfraType::TickerPlant).await.unwrap();
        if let Err(e) = handle_received_responses(rithmic_api_clone, ticker_receiver, SysInfraType::TickerPlant).await {
            eprintln!("Error handling ticker responses: {:?}", e);
        }
    });*/

  /*  // Spawn a task to handle HistoryPlant responses
    let rithmic_api_clone = rithmic_api_arc.clone();
    task::spawn(async move {
        let history_receiver = rithmic_api_clone.connect_and_login(SysInfraType::HistoryPlant).await.unwrap();
        if let Err(e) = handle_received_responses(rithmic_api_clone, history_receiver, SysInfraType::HistoryPlant).await {
            eprintln!("Error handling history responses: {:?}", e);
        }
    });*/

    // Spawn a task to handle OrderPlant responses
    let rithmic_api_clone = rithmic_api_arc.clone();
    task::spawn(async move {
        let order_receiver = rithmic_api_clone.connect_and_login(SysInfraType::TickerPlant).await.unwrap();
        if let Err(e) = handle_received_responses(rithmic_api_clone, order_receiver, SysInfraType::TickerPlant).await {
            eprintln!("Error handling order responses: {:?}", e);
        }
    });

/*    // Spawn a task to handle PnlPlant responses
    let rithmic_api_clone = rithmic_api_arc.clone();
    task::spawn(async move {
        let pnl_receiver = rithmic_api_clone.connect_and_login(SysInfraType::PnlPlant).await.unwrap();
        if let Err(e) = handle_received_responses(rithmic_api_clone, pnl_receiver, SysInfraType::PnlPlant).await {
            eprintln!("Error handling pnl responses: {:?}", e);
        }
    });*/



    /*    // send a heartbeat request as a test message, 'RequestHeartbeat' Template number 18
        let heart_beat = RequestHeartbeat {
            template_id: 18,
            user_msg: vec![format!("{} Testing heartbeat", app_name)],
            ssboe: None,
            usecs: None,
        };*/



    // We can send messages with only a reference to the client, so we can wrap our client in Arc or share it between threads and still utilise all associated functions.


    // Keep the main task alive, e.g., by using an infinite loop or sleeping for a long period
    loop {
     /*   let req = RequestProductRmsInfo {
            template_id: 306,
            user_msg: vec!["callback_id".to_string()],
            fcm_id: None,
            ib_id: Some("NQ".to_string()),
            account_id: None,
        };*/
        /*let req = RequestAccountList {
            template_id: 302,
            user_msg: vec!["callback_id".to_string()],
            fcm_id: None,
            ib_id: None,
            user_type: None,
        };*/
        let req = RequestGiveTickSizeTypeTable {
            template_id: 107,
            user_msg: vec!["callback_id".to_string()],
            tick_size_type: Some("1".to_string()),
        };

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        match rithmic_api_arc.send_message(&SysInfraType::TickerPlant, &req).await {
            Ok(_) => {},
            Err(e) => eprintln!("{}", e)
        }
    }


    // Logout and Shutdown all connections
    rithmic_api_arc.shutdown_all().await.unwrap();
}