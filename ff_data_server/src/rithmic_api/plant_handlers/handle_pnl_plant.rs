use std::sync::Arc;
#[allow(unused_imports)]
use ff_rithmic_api::credentials::RithmicCredentials;
#[allow(unused_imports)]
use ff_rithmic_api::rithmic_proto_objects::rti::{AccountListUpdates, AccountPnLPositionUpdate, AccountRmsUpdates, BestBidOffer, BracketUpdates, DepthByOrder, DepthByOrderEndEvent, EndOfDayPrices, ExchangeOrderNotification, FrontMonthContractUpdate, IndicatorPrices, InstrumentPnLPositionUpdate, LastTrade, MarketMode, OpenInterest, OrderBook, OrderPriceLimits, QuoteStatistics, RequestAccountList, RequestAccountRmsInfo, RequestHeartbeat, RequestLoginInfo, RequestMarketDataUpdate, RequestPnLPositionSnapshot, RequestPnLPositionUpdates, RequestProductCodes, RequestProductRmsInfo, RequestReferenceData, RequestTickBarUpdate, RequestTimeBarUpdate, RequestVolumeProfileMinuteBars, ResponseAcceptAgreement, ResponseAccountList, ResponseAccountRmsInfo, ResponseAccountRmsUpdates, ResponseAuxilliaryReferenceData, ResponseBracketOrder, ResponseCancelAllOrders, ResponseCancelOrder, ResponseDepthByOrderSnapshot, ResponseDepthByOrderUpdates, ResponseEasyToBorrowList, ResponseExitPosition, ResponseFrontMonthContract, ResponseGetInstrumentByUnderlying, ResponseGetInstrumentByUnderlyingKeys, ResponseGetVolumeAtPrice, ResponseGiveTickSizeTypeTable, ResponseHeartbeat, ResponseLinkOrders, ResponseListAcceptedAgreements, ResponseListExchangePermissions, ResponseListUnacceptedAgreements, ResponseLogin, ResponseLoginInfo, ResponseLogout, ResponseMarketDataUpdate, ResponseMarketDataUpdateByUnderlying, ResponseModifyOrder, ResponseModifyOrderReferenceData, ResponseNewOrder, ResponseOcoOrder, ResponseOrderSessionConfig, ResponsePnLPositionSnapshot, ResponsePnLPositionUpdates, ResponseProductCodes, ResponseProductRmsInfo, ResponseReferenceData, ResponseReplayExecutions, ResponseResumeBars, ResponseRithmicSystemInfo, ResponseSearchSymbols, ResponseSetRithmicMrktDataSelfCertStatus, ResponseShowAgreement, ResponseShowBracketStops, ResponseShowBrackets, ResponseShowOrderHistory, ResponseShowOrderHistoryDates, ResponseShowOrderHistoryDetail, ResponseShowOrderHistorySummary, ResponseShowOrders, ResponseSubscribeForOrderUpdates, ResponseSubscribeToBracketUpdates, ResponseTickBarReplay, ResponseTickBarUpdate, ResponseTimeBarReplay, ResponseTimeBarUpdate, ResponseTradeRoutes, ResponseUpdateStopBracketLevel, ResponseUpdateTargetBracketLevel, ResponseVolumeProfileMinuteBars, RithmicOrderNotification, SymbolMarginRate, TickBar, TimeBar, TradeRoute, TradeStatistics, UpdateEasyToBorrowList};
use ff_rithmic_api::rithmic_proto_objects::rti::Reject;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use prost::{Message as ProstMessage};
#[allow(unused_imports)]
use ff_standard_lib::standardized_types::broker_enum::Brokerage;
use crate::rithmic_api::api_client::RithmicClient;

#[allow(dead_code, unused)]
pub async fn match_pnl_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    client: Arc<RithmicClient>
) {
    const PLANT: SysInfraType = SysInfraType::PnlPlant;
    match template_id {
        75 => {
            if let Ok(msg) = Reject::decode(&message_buf[..]) {
                // Login Response
                // From Server
                println!("Reject Response (Template ID: 11) from Server: {:?}", msg);
            }
        }
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
                //println!("Response Heartbeat (Template ID: 19) from Server: {:?}", msg);
                client.handle_response_heartbeat(PLANT, msg);
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
