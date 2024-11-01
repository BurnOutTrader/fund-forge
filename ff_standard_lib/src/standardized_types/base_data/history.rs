use crate::helpers::converters::next_month;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::messages::data_server_messaging::{
    DataServerRequest, DataServerResponse, FundForgeError,
};
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use chrono::{DateTime, Duration, Utc};
use std::collections::{BTreeMap, HashMap};
use tokio::sync::oneshot;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::standardized_types::enums::{StrategyMode, SubscriptionResolutionType};
use crate::strategies::client_features::request_handler::{send_request, StrategyRequest};
use crate::strategies::consolidators::consolidator_enum::ConsolidatorEnum;

pub async fn get_historical_data(
    subscriptions: Vec<DataSubscription>,
    from_time: DateTime<Utc>,
    to_time: DateTime<Utc>,
) -> Result<BTreeMap<i64, TimeSlice>, FundForgeError> {
    let req = DataServerRequest::HistoricalBaseDataRange {
        callback_id: 0,
        subscriptions: subscriptions.clone(),
        from_time: from_time.to_string(),
        to_time: to_time.to_string(),
    };
    let (tx, rx) = oneshot::channel();
    let request = StrategyRequest::CallBack(
        ConnectionType::Default,
        req,
        tx
    );

    // Send request sequentially
    send_request(request).await;
    let response = rx.await;
    match response {
        Ok(payload ) => {
            match payload {
                DataServerResponse::HistoricalBaseData { payload, .. } => Ok(payload),
                DataServerResponse::Error { error, .. } => {
                    panic!("{}", error)
                },
                _ => panic!("Incorrect callback data received")
            }
        }
        Err(e) => panic!("Failed to receive payload: {}", e)
    }
}

pub fn get_lowest_resolution(
    all_symbol_subscriptions: &HashMap<Symbol, Vec<DataSubscription>>,
    symbol: &Symbol,
) -> Option<Resolution> {
    all_symbol_subscriptions
        .get(symbol)
        .and_then(|subscriptions| {
            subscriptions
                .iter()
                .min_by_key(|sub| &sub.resolution)
                .map(|sub| sub.resolution.clone())
        })
}


pub async fn range_history_data(
    from_time: DateTime<Utc>,
    to_time: DateTime<Utc>,
    subscription: DataSubscription,
    mode: StrategyMode
) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
    if from_time > to_time {
        panic!("From time cannot be greater than to time");
    }
    let sub_res_type = SubscriptionResolutionType::new(subscription.resolution, subscription.base_data_type);
    let resolutions = subscription.symbol.data_vendor.resolutions(subscription.symbol.market_type).await.unwrap();
    if resolutions.contains(&sub_res_type) {
        let data = match get_historical_data(vec![subscription.clone()], from_time, to_time).await {
            Ok(data) => {
                data
            }
            Err(e) => {
                eprintln!("No data available or error: {}", e);
                return BTreeMap::new()
            }
        };
        let mut map:BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
        for (_, slice) in data {

            for base_data in slice.iter() {
                let data_time = base_data.time_closed_utc();
                if  data_time < from_time || data_time > to_time {
                    continue
                }
                if base_data.subscription() == subscription {}
                map.insert(base_data.time_utc(), base_data.clone());
            }
        }
        map
    } else {
        let duration = to_time - from_time - Duration::days(3);
        let duration_ns = duration.num_nanoseconds().unwrap();  // Total nanoseconds in `duration`
        let resolution_ns = subscription.resolution.as_duration().num_nanoseconds().unwrap(); // Total nanoseconds in `resolution`

        let history_to_retain = duration_ns / resolution_ns;
        let consolidator = ConsolidatorEnum::create_consolidator(subscription, false).await;
        let (_, window) = ConsolidatorEnum::warmup(consolidator, to_time, history_to_retain as i32, mode).await;
        let mut map:BTreeMap<DateTime<Utc>, BaseDataEnum> = BTreeMap::new();
        for base_data in window.history() {
            let data_time = base_data.time_closed_utc();
            if  data_time < from_time || data_time > to_time {
                continue
            }
            map.insert(data_time, base_data.clone());
        }
        map
    }
}

pub fn generate_file_dates(
    mut start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> BTreeMap<i32, DateTime<Utc>> {
    let mut month_years: BTreeMap<i32, DateTime<Utc>> = BTreeMap::new();
    let mut msg_count = 0;
    while &start_time < &end_time {
        msg_count += 1;
        month_years.insert(msg_count, start_time.clone());
        start_time = next_month(&start_time.clone());
    }
    month_years
}
