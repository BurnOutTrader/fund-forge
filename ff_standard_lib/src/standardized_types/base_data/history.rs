use crate::helpers::converters::next_month;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::{
    BaseDataPayload, FundForgeError, DataServerRequest, DataServerResponse,
};
use crate::standardized_types::enums::Resolution;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::time_slices::TimeSlice;
use crate::standardized_types::time_slices::UnstructuredSlice;
use chrono::{DateTime, Utc};
use std::collections::{btree_map, BTreeMap, Bound, HashMap};
use tokio::sync::oneshot;
use crate::server_connections::{get_sender, ConnectionType, StrategyRequest};

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
    if slices.len() == 0 {
        eprintln!("No TimeSlices to process");
        return None;
    }

    let mut time_slices: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
    // Deserialize and get time for sorting
    for unstructured_slice in slices {
        let base_data: Vec<BaseDataEnum> =
            match BaseDataEnum::from_array_bytes(&unstructured_slice.data) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Engine: Failed to deserialize bytes array: {}", e);
                    return None
                },
            };

        for data in base_data {
            let time = data.time_created_utc();
            if let btree_map::Entry::Vacant(e) = time_slices.entry(time) {
                let time_slice: TimeSlice = vec![data];
                e.insert(time_slice);
            } else if let Some(time_slice) = time_slices.get_mut(&time) {
                time_slice.push(data);
            }
        }
    }
    Some(time_slices)
}

pub fn build_data_que(base_data: Vec<BaseDataEnum>) -> BTreeMap<DateTime<Utc>, TimeSlice> {
    let mut time_slices: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
    for data in base_data {
        let time = data.time_created_utc();
        if let btree_map::Entry::Vacant(e) = time_slices.entry(time) {
            let time_slice: TimeSlice = vec![data];
            e.insert(time_slice);
        } else if let Some(time_slice) = time_slices.get_mut(&time) {
            time_slice.push(data);
        }
    }
    time_slices
}

/// Method responsible for getting historical data for a single subscription
/// Can be used to get historical data for a single subscription.
/// Is useful for warming up the strategy and getting historical data for a single subscription.
/// Can be used to manually warm up indicators during re-balancing or adding new subscriptions. since this is not done Automatically.
///
/// # Important Note
/// This method only gets 1 month data based on the specified time, if you need more data, you will need to call the method again with the next month, this is a backend operation.
/// It does not return a single data point but a 1 month file of data points.
/// To get specific history range use the `history` method.
///
/// # Arguments
/// * `time` - The time to get the historical data for
/// * `subscriptions` - The `Vec<Subscription` subscriptions to get the historical data for
///
/// # Returns
/// BTreeMap<i64, TimeSlice> - where i64 is the time and TimeSlice is the data for that time
/// The data is sorted by time and the time is the key, a TimeSlice is a vector of slices that occurred at that time, so if multiple symbols created a bar at the same time, they will be in the same TimeSlice.
/// For this reason you can subscribe to resolutions of multiple symbols and time frames and the data will be sorted and consolidated into TimeSlice objects.
///
/// # Note
/// If you are going to update or warm up multiple subscriptions, it is more efficient and appropriate to use the `get_historical_data_all_subscriptions` method. Since it will return TimeSlice objects for all subscriptions at the same time.
/// Allowing you to update all subscriptions at the same time more accurately and efficiently.
/// This method is for when you only want to get historical data for a single subscription.
/// If the data is available with the vendor but not the server, it will be downloaded, this can take a long time, the data will continue to download so long as the server is running, even after the strategy stops.
pub async fn get_historical_data(
    subscriptions: Vec<DataSubscription>,
    time: DateTime<Utc>,
) -> Result<BTreeMap<DateTime<Utc>, TimeSlice>, FundForgeError> {

    // we cant use the HistoricalBaseDataMany variant since we might have a unique api for each vendor, therefore we need to send the messages async here and collect the futures.
    let futures: Vec<_> = subscriptions.iter().map(|sub| {
        let sub = sub.clone();
        async move {
            let (sender, receiver) = oneshot::channel();
            let request: StrategyRequest = StrategyRequest::CallBack(
                    ConnectionType::Vendor(sub.symbol.data_vendor.clone()),
                    DataServerRequest::HistoricalBaseData {
                    callback_id: 0,
                    subscription: sub.clone(),
                    time: time.to_string(),
                },
                sender
            );
            let sender = get_sender();
            let sender = sender.lock().await;
            sender.send(request).await.unwrap();

           let response = receiver.await.unwrap();
            match response {
                DataServerResponse::HistoricalBaseData{payload, ..} => {
                    Ok(payload)
                }
                DataServerResponse::Error{error, ..} => Err(FundForgeError::ClientSideErrorDebug(format!("Error: {}", error))),
                _ => Err(FundForgeError::ClientSideErrorDebug("Incorrect Response on callback".to_string())),
            }
        }
    }).collect();

    let mut results:Vec<Result<BaseDataPayload, FundForgeError>> = futures::future::join_all(futures).await;

    let mut payloads: Vec<BaseDataPayload> = vec![];

    for result in results.iter_mut() {
        match result {
            Ok(data) => payloads.push(data.to_owned()),
            Err(e) => eprintln!("Engine: Failed to get data: {:?}", e),
        }
    }

    let mut slices: Vec<UnstructuredSlice> = Vec::new();

    // convert our payloads into unstructured slices
    for payload in payloads {
        let slice = UnstructuredSlice {
            data: payload.bytes,
            resolution: payload.subscription.resolution,
            price_data_type: payload.subscription.base_data_type,
            symbol: payload.subscription.symbol,
        };
        slices.push(slice);
    }

    // structure our slices into time slices and arrange by time
    let tree = match structured_que_builder(slices).await {
        Some(time_slices) => Ok(time_slices),
        None => Err(FundForgeError::ClientSideErrorDebug(
            "Error getting historical data for all subscriptions".to_string(),
        )),
    };

    // return the time slices as a BTreeMap ordered by time
    tree
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
    subscription: DataSubscription,
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
