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
pub async fn match_history_plant_id(
    template_id: i32, message_buf: Vec<u8>,
    _client: Arc<RithmicClient>,
) {
    const PLANT: SysInfraType = SysInfraType::HistoryPlant;
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