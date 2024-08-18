use crate::standardized_types::time_slices::TimeSlice;
use crate::standardized_types::time_slices::UnstructuredSlice;
use std::collections::{btree_map, BTreeMap};
use chrono::{DateTime, FixedOffset, Utc};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::{BaseDataPayload, FundForgeError, SynchronousRequestType, SynchronousResponseType};
use crate::standardized_types::subscriptions::DataSubscription;
use crate::helpers::converters::next_month;


/// Method responsible for getting historical data for a specific subscription.
/// Users should use this method to get historical data for a specific subscription/subscriptions
///
/// This method can be used to get a `Option<Vec<TimeSlice>>` for a `Subscription` from the `ff_data_server` instance.
/// It could be used to get the historical data and view a symbol without actually subscribing the algorithm.
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
async fn que_builder(slices: Vec<UnstructuredSlice>) -> Option<BTreeMap<DateTime<Utc>, TimeSlice>> {
    let mut time_slices: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();

    if slices.len() == 0 {
        println!("No TimeSlices to process");
        return None;
    }

    // Deserialize and get time for sorting
    for unstructured_slice in slices {
        let price_data: Vec<BaseDataEnum> = match BaseDataEnum::from_array_bytes(&unstructured_slice.data) {
            Ok(data) => data,
            Err(_) => return None
        };

        for data in price_data {
            let time = data.time_created_utc();
            if let btree_map::Entry::Vacant(e) = time_slices.entry(time) {
                let time_slice : TimeSlice = vec![data];
                e.insert(time_slice);
            } else if let Some(time_slice) = time_slices.get_mut(&time) {
                time_slice.push(data);
            }
        }
    }
    Some(time_slices)
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
pub async fn get_historical_data(subscriptions: Vec<DataSubscription>, time: DateTime<Utc>) -> Result<BTreeMap<DateTime<Utc>, TimeSlice>, FundForgeError> {
    let mut payloads : Vec<BaseDataPayload> = Vec::new();

    for sub in &subscriptions {
        let request : SynchronousRequestType = SynchronousRequestType::HistoricalBaseData {
            subscriptions: sub.clone(),
            time: time.to_string(),
        };
        let synchronous_communicator = sub.symbol.data_vendor.synchronous_client().await;

        let response = synchronous_communicator.send_and_receive(request.to_bytes(), false).await;

        match response {
            Ok(response) => {
                let response = SynchronousResponseType::from_bytes(&response).unwrap();
                match response {
                    SynchronousResponseType::HistoricalBaseData(payload) => payloads.extend(payload),
                    SynchronousResponseType::Error(e) => panic!("Error: {}", e),
                    _ => panic!("Invalid response type from server"),
                }
            },
            Err(e) => panic!("Error: {}", e),
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
    let tree = match que_builder(slices).await {
        Some(time_slices) => Ok(time_slices),
        None => Err(FundForgeError::ClientSideErrorDebug("Error getting historical data for all subscriptions".to_string()))
    };

    // return the time slices as a BTreeMap ordered by time
    tree
}


/// Method responsible for getting historical data for all subscriptions that are passed in,
/// # Arguments
/// * `subscriptions` - The subscriptions to get the historical data for, this doesn't actually subscribe your strategy to the symbol, it just defines the request we are making to get the correct data. This is a `Vec<Subscription>` objects.
/// * `from_time` - The start time to get the historical data from, this is a `chrono::naive::datetime:NaiveDateTime` object.
/// * `to_time` - The end time to get the historical data to, this is a `chrono::naive::datetime:NaiveDateTime` object.
///
/// # Returns
/// `Option<BTreeMap<i64, TimeSlice>>` - where `i64` is the `time` and `TimeSlice` is the data for that time.
/// The TimeSlice is a type `Vec<BaseDataEnum>` that occurred at that moment in time. any kind of `BaseDataEnum` can be mixed into this list. \
pub async fn history(subscriptions: Vec<DataSubscription>, from_time: DateTime<FixedOffset>, to_time: DateTime<FixedOffset>) -> Option<BTreeMap<DateTime<Utc>, TimeSlice>> {
    let from_time = from_time.to_utc();
    let to_time = to_time.to_utc();

    let month_years = generate_file_dates(from_time.clone(), to_time.clone());

    let mut last_time = from_time.clone();
    let end_time: DateTime<Utc> = to_time.clone();

    let mut data : BTreeMap<DateTime<Utc>, TimeSlice>  = BTreeMap::new();

    // While the last time is less than the strategy start time, we keep getting data and feeding it to the algo.
    let mut end_loop = false;
    for (_, month_year) in month_years{ //start time already utc
        //println!("Getting historical data for: {:?}", month_year);
        // Get the historical data for all subscriptions from the server, parse it into TimeSlices.
        let time_slices = match get_historical_data(subscriptions.clone(), month_year).await {
            Ok(time_slices) => {
                time_slices
            },
            Err(_e) =>continue
        };

        if time_slices.len() == 0 {
            continue;
        }

        // We loop through the time slices and get the data that is in the range we want.
        for (time, time_slice) in time_slices {
            if time < from_time {
                continue;
            }

            if time >= end_time {
                end_loop = true;
                break;
            }
            if time <= last_time {
                continue;
            }
            last_time = time.clone();
           data.insert(time, time_slice);
        }
        if end_loop {
            break;
        }
    }
    if data.len() == 0 {
        return None;
    }
    Some(data)
}

pub fn generate_file_dates(mut start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> BTreeMap<i32, DateTime<Utc>> {
    let mut month_years : BTreeMap<i32, DateTime<Utc>> = BTreeMap::new();
    let mut msg_count = 0;
    while &start_time < &end_time {
        msg_count += 1;
        month_years.insert(msg_count, start_time.clone());
        start_time = next_month(&start_time.clone());
    }
    month_years
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::vendor::DataVendor;
    use crate::server_connections::{initialize_clients, PlatformMode};
    use crate::standardized_types::base_data::base_data_type::BaseDataType;
    use crate::standardized_types::enums::{MarketType, Resolution};

     #[tokio::test]
       async fn test_initialize_clients_single_machine() {
            let result = initialize_clients(&PlatformMode::SingleMachine).await;
            assert!(result.is_ok());

             let subscription = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex);
             let time = DateTime::from_timestamp(1612473600, 0).unwrap();
             match get_historical_data(vec![subscription.clone()], time).await {
                 Ok(time_slices) => {
                     //println!("{:?}", time_slices);
                     assert!(time_slices.len() > 0);
                 },
                 Err(e) => {
                     panic!("Error: {}", e);
                 }
             }

             let result = initialize_clients(&PlatformMode::SingleMachine).await;
             assert!(result.is_ok());

             let subscription = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex);
             let time = DateTime::from_timestamp(1612473600, 0).unwrap();
             match get_historical_data(vec![subscription.clone()], time).await {
                 Ok(time_slices) => {
                     assert!(time_slices.len() > 0);
                 },
                 Err(e) => {
                     panic!("Error: {}", e);
                 }
             }
        }

    #[tokio::test]
    async fn test_get_historical_data() {
        // Assuming you have a way to mock or set up the settings for a multi-machine scenario
        let result = initialize_clients(&PlatformMode::MultiMachine).await;
        assert!(result.is_ok());

        let subscription = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex);
        let time = DateTime::from_timestamp(1612473600, 0).unwrap();
        match get_historical_data(vec![subscription.clone()], time).await {
            Ok(time_slices) => {
                //println!("{:?}", time_slices);
                assert!(time_slices.len() > 0);
            },
            Err(e) => {
                panic!("Error: {}", e);
            }
        }

        let result = initialize_clients(&PlatformMode::SingleMachine).await;
        assert!(result.is_ok());

        let subscription = DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex);
        let time = DateTime::from_timestamp(1612473600, 0).unwrap();
        match get_historical_data(vec![subscription.clone()], time).await {
            Ok(time_slices) => {
                assert!(time_slices.len() > 0);
            },
            Err(e) => {
                panic!("Error: {}", e);
            }
        }
    }
}

