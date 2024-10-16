use std::sync::Arc;
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use prost::{Message as ProstMessage};
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use crate::rithmic_api::api_client::RithmicClient;

#[allow(dead_code)]
pub async fn match_repo_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    _client: Arc<RithmicClient>,
) {
    const PLANT: SysInfraType = SysInfraType::RepositoryPlant;
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