use crate::helpers::converters::next_month;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::messages::data_server_messaging::{
    BaseDataPayload, DataServerRequest, DataServerResponse, FundForgeError,
};
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use crate::standardized_types::time_slices::UnstructuredSlice;
use chrono::{DateTime, Duration, Utc};
use std::collections::{BTreeMap, Bound, HashMap};
use tokio::sync::oneshot;
use crate::strategies::client_features::server_connections::{send_request, StrategyRequest};
use dashmap::DashMap;
use rayon::prelude::*;
use crate::strategies::client_features::connection_types::ConnectionType;
use futures::future::join_all;
use crate::standardized_types::enums::{StrategyMode, SubscriptionResolutionType};
use crate::strategies::consolidators::consolidator_enum::ConsolidatorEnum;

/// Method responsible for structuring raw historical data into combined time slices, where all data points a combined into BTreeMap<DateTime<Utc>, TimeSlice> where data.time_created() is key and value is TimeSlice.
///
/// # Arguments
/// * `subscriptions` - The subscriptions to get the historical data for, this doesn't actually subscribe your strategy to the symbol, it just defines the request we are making to get the correct data. This is a `Vec<Subscription>` objects.
/// * `from_time` - The start time to get the historical data from, this is a `chrono::naive::datetime:NaiveDateTime` object.
/// * `to_time` - The end time to get the historical data to, this is a `chrono::naive::datetime:NaiveDateTime` object.
/// * `client` - The client to use to connect to the ff_data_server, this is an `Arc<Mutex<TcpStream>>` object.
///
/// # Returns
/// `Option<Vec<TimeSlice>>` - A vector of `TimeSlice` objects, each `TimeSlice` contains a `time` and a `Vec<Slice>`. The `time` is the time the data was recorded and the `Vec<Slice>` contains the data for that time.
/// The data is sorted by time and the time is the key, a `TimeSlice` is a vector of slices that occurred at that time, so if multiple symbols created a bar at the same time, they will be in the same `TimeSlice`.
/// For this reason you can subscribe to resolutions of multiple symbols and time frames and the data will be sorted and consolidated into `TimeSlice` objects.
///
/// # Note
/// If No history is available for the specified times, the method will return `None`.
/// If the data is available with the vendor but not the server, it will be downloaded, this can take a long time, the data will continue to download so long as the server is running, even after the strategy stops.
/// Method responsible for consolidating and formatting data into TimeSlice objects.
///
/// # Arguments
/// * `slices` - An vector of `UnstructuredSlice` objects, each `UnstructuredSlice` contains a `time`, `interval`, `PriceDataType` enum and `symbol`. The data is mixed and unsorted by `time` and `symbol`.
///
/// # Returns
/// `Option<BTreeMap<i64, TimeSlice>>` - where `i64` is the `time` and `TimeSlice` is the data for that time.
/// The TimeSlice is a type `Vec<BaseDataEnum>` that occurred at that moment in time. any kind of of `BaseDataEnum` can be mixed into this list.
/// ```rust
/// ```
async fn structured_que_builder(
    slices: Vec<UnstructuredSlice>,
) -> Option<BTreeMap<DateTime<Utc>, TimeSlice>> {
    if slices.is_empty() {
        eprintln!("No TimeSlices to process");
        return None;
    }

    let time_slices: DashMap<DateTime<Utc>, TimeSlice> = DashMap::new();

    // Use Rayon for parallel processing of the slices
    slices.par_iter().for_each(|unstructured_slice| {
        let base_data: Vec<BaseDataEnum> =
            match BaseDataEnum::from_array_bytes(&unstructured_slice.data) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Engine: Failed to deserialize bytes array: {}", e);
                    return;
                },
            };

        for data in base_data {
            let time = data.time_closed_utc();
            if let Some(mut time_slice) = time_slices.get_mut(&time) {
                time_slice.add(data)
            } else {
                let mut time_slice = TimeSlice::new();
                time_slice.add(data);
                time_slices.insert(time, time_slice);
            }
        }
    });

    // Convert DashMap to BTreeMap (sorted by time key)
    let mut final_map: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
    for (key, entry) in time_slices.into_iter() {
        final_map.insert(key, entry);
    }

    Some(final_map)
}



pub async fn get_historical_data(
    subscriptions: Vec<DataSubscription>,
    time: DateTime<Utc>,
) -> Result<BTreeMap<DateTime<Utc>, TimeSlice>, FundForgeError> {

    // Store the receivers of the futures for later concurrent awaiting
    let mut receivers = Vec::new();

    // Send requests one by one (sequentially)
    for sub in subscriptions {
        let sub = sub.clone();

        // Create the oneshot channel for response
        let (tx, rx) = oneshot::channel();

        let request = StrategyRequest::CallBack(
            ConnectionType::Vendor(sub.symbol.data_vendor.clone()),
            DataServerRequest::HistoricalBaseData {
                callback_id: 0,
                subscription: sub.clone(),
                time: time.to_string(),
            },
            tx,
        );

        // Send request sequentially
        send_request(request).await;

        // Collect the receiver (rx) for concurrent awaiting later
        receivers.push(rx);
    }

    // Await all responses concurrently
    let mut payloads: Vec<BaseDataPayload> = Vec::new();
    let responses = join_all(receivers.into_iter().map(|rx| async {
        match rx.await {
            Ok(DataServerResponse::HistoricalBaseData { payload, .. }) => Ok(payload),
            Ok(DataServerResponse::Error { error, .. }) => Err(FundForgeError::ClientSideErrorDebug(format!("Error: {}", error))),
            Ok(_) | Err(_) => Err(FundForgeError::ClientSideErrorDebug("Incorrect or failed callback response".to_string())),
        }
    }))
        .await;

    // Process each response
    for result in responses {
        match result {
            Ok(data) => payloads.push(data),
            Err(e) => eprintln!("Engine: Failed to get data: {:?}", e),
        }
    }

    // Convert the payloads into UnstructuredSlice
    let slices: Vec<UnstructuredSlice> = payloads
        .into_iter()
        .map(|payload| UnstructuredSlice {
            data: payload.bytes,
            resolution: payload.subscription.resolution,
            price_data_type: payload.subscription.base_data_type,
            symbol: payload.subscription.symbol,
        })
        .collect();

    // Structure our slices into time slices and arrange by time
    match structured_que_builder(slices).await {
        Some(time_slices) => Ok(time_slices),
        None => Err(FundForgeError::ClientSideErrorDebug(
            "Error getting historical data for all subscriptions".to_string(),
        )),
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

pub async fn range_data(
    from_time: DateTime<Utc>,
    to_time: DateTime<Utc>,
    subscription: DataSubscription
) -> BTreeMap<DateTime<Utc>, TimeSlice> {
    if from_time > to_time {
        panic!("From time cannot be greater than to time");
    }
    let month_years = generate_file_dates(from_time.clone(), to_time.clone());

    let mut data: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
    for (_, month_year) in month_years {
        //start time already utc
        //println!("Getting historical data for: {:?}", month_year);
        // Get the historical data for all subscriptions from the server, parse it into TimeSlices.
        let time_slices = match get_historical_data(vec![subscription.clone()], month_year).await {
            Ok(time_slices) => time_slices,
            Err(_e) => continue,
        };

        if time_slices.len() == 0 {
            continue;
        }

        let range = (Bound::Included(from_time), Bound::Included(to_time));
        data.extend(
            time_slices
                .range(range)
                .map(|(k, v)| (k.clone(), v.clone())),
        );
    }
    data
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
        let month_years = generate_file_dates(from_time.clone(), to_time.clone());

        let mut data: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
        for (_, month_year) in month_years {
            //start time already utc
            //println!("Getting historical data for: {:?}", month_year);
            // Get the historical data for all subscriptions from the server, parse it into TimeSlices.
            let time_slices = match get_historical_data(vec![subscription.clone()], month_year).await {
                Ok(time_slices) => time_slices,
                Err(_e) => continue,
            };

            if time_slices.len() == 0 {
                continue;
            }

            let range = (Bound::Included(from_time), Bound::Included(to_time));
            data.extend(
                time_slices
                    .range(range)
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
        }
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
        let consolidator = ConsolidatorEnum::create_consolidator(subscription, false, sub_res_type).await;
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
