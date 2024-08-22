use crate::standardized_types::time_slices::TimeSlice;
use crate::standardized_types::time_slices::UnstructuredSlice;
use std::collections::{btree_map, BTreeMap, HashMap};
use chrono::{DateTime, FixedOffset, Utc};
use futures::future::join_all;
use iced::Application;
use crate::apis::vendor::client_requests::ClientSideDataVendor;
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::data_server_messaging::{BaseDataPayload, FundForgeError, SynchronousRequestType, SynchronousResponseType};
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::helpers::converters::next_month;
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::enums::Resolution;
use crate::subscription_handler::ConsolidatorEnum;

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
async fn unstructured_que_builder(slices: Vec<UnstructuredSlice>) -> Option<BTreeMap<DateTime<Utc>, TimeSlice>> {
    if slices.len() == 0 {
        println!("No TimeSlices to process");
        return None;
    }

    let mut time_slices: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
    // Deserialize and get time for sorting
    for unstructured_slice in slices {
        let base_data: Vec<BaseDataEnum> = match BaseDataEnum::from_array_bytes(&unstructured_slice.data) {
            Ok(data) => data,
            Err(_) => return None
        };

        for data in base_data {
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

pub fn build_data_que(base_data: Vec<BaseDataEnum> ) -> BTreeMap<DateTime<Utc>, TimeSlice> {
    let mut time_slices: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
    for data in base_data {
        let time = data.time_created_utc();
        if let btree_map::Entry::Vacant(e) = time_slices.entry(time) {
            let time_slice : TimeSlice = vec![data];
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
    let tree = match unstructured_que_builder(slices).await {
        Some(time_slices) => Ok(time_slices),
        None => Err(FundForgeError::ClientSideErrorDebug("Error getting historical data for all subscriptions".to_string()))
    };

    // return the time slices as a BTreeMap ordered by time
    tree
}

pub fn get_lowest_resolution(all_symbol_subscriptions: &HashMap<Symbol, Vec<DataSubscription>>, symbol: &Symbol) -> Option<Resolution> {
    all_symbol_subscriptions.get(symbol).and_then(|subscriptions| {
        subscriptions.iter().min_by_key(|sub| &sub.resolution).map(|sub| sub.resolution.clone())
    })
}


pub async fn history_many(subscriptions: Vec<DataSubscription>, from_time: DateTime<FixedOffset>, to_time: DateTime<FixedOffset>) -> Option<BTreeMap<DateTime<Utc>, TimeSlice>> {
    let mut combined_data: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();

    // Create a list of futures for concurrent execution
    let history_futures = subscriptions.into_iter()
        .map(|subscription| history(subscription, from_time, to_time))
        .collect::<Vec<_>>();

    // Await all the futures concurrently
    let results = join_all(history_futures).await;

    for history_result in results {
        match history_result {
            Some(history) => {
                for (time, slice) in history {
                    if !combined_data.contains_key(&time) {
                        combined_data.insert(time.clone(), slice);
                    } else {
                        combined_data.get_mut(&time).unwrap().extend(slice);
                    }
                }
            },
            None => return None,
        }
    }

    if combined_data.is_empty() {
        None
    } else {
        Some(combined_data)
    }
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
pub async fn history(subscription: DataSubscription, from_time: DateTime<FixedOffset>, to_time: DateTime<FixedOffset>) -> Option<BTreeMap<DateTime<Utc>, TimeSlice>> {
    if from_time > to_time {
        panic!("From time cannot be greater than to time");
    }
    //determine if the subscription is a base_resolution
    let vendor_resolutions = subscription.symbol.data_vendor.resolutions(subscription.market_type.clone()).await.unwrap();
    if vendor_resolutions.contains(&subscription.resolution) {
        let data = range_data(from_time.to_utc(), to_time.to_utc(), subscription.clone()).await;
        match data.is_empty() {
            false => return Some(data),
            true => return None
        }
    } else {
        //determine the correct resolution
        let mut minimum_resolution: Option<Resolution> = None;
         for resolution in vendor_resolutions {
             if minimum_resolution.is_none() {
                 minimum_resolution = Some(resolution);
             } else {
                 if resolution > minimum_resolution.unwrap() && resolution < subscription.resolution {
                     minimum_resolution = Some(resolution);
                 }
             }
        }
        let minimum_resolution = match minimum_resolution.is_none() {
            true => return None,
            false => minimum_resolution.unwrap()
        };

        let data_type = match minimum_resolution {
            Resolution::Ticks(_) => BaseDataType::Ticks,
            _ => subscription.base_data_type.clone()
        };
        
        let base_subscription = DataSubscription::new(subscription.symbol.name.clone(), subscription.symbol.data_vendor.clone(), minimum_resolution, data_type, subscription.market_type.clone());
        let base_data = range_data(from_time.to_utc(), to_time.to_utc(), base_subscription.clone()).await;

        let mut consolidator = match subscription.resolution {
            Resolution::Ticks(_) => ConsolidatorEnum::new_count_consolidator(subscription.clone(), 1).unwrap(),
            _ => ConsolidatorEnum::new_time_consolidator(subscription.clone(), 1).unwrap()
        };
        let mut consolodated_data: BTreeMap<DateTime<Utc>, TimeSlice> = BTreeMap::new();
        for (time, slice) in base_data {
            for data in slice {
                let new_consolidated = consolidator.update(&data);
                if !new_consolidated.is_empty() {
                    for consolidated in new_consolidated{
                        if consolidated.time_created_utc() < from_time.to_utc() {
                            continue;
                        }
                        if !consolodated_data.contains_key(&consolidated.time_created_utc()) {
                            consolodated_data.insert(time.clone(), vec![consolidated]);
                        } else {
                            consolodated_data.get_mut(&consolidated.time_created_utc()).unwrap().push(consolidated);
                        }
                    }
                }
            }
        }
        match consolodated_data.is_empty() {
            false => return Some(consolodated_data),
            true => return None
        }
    }
}

pub(crate) async fn range_data(from_time: DateTime<Utc>, to_time: DateTime<Utc>, subscription: DataSubscription) -> BTreeMap<DateTime<Utc>, TimeSlice> {
    if from_time > to_time {
        panic!("From time cannot be greater than to time");
    }
    
    let month_years = generate_file_dates(from_time.clone(), to_time.clone());
    
    let mut data : BTreeMap<DateTime<Utc>, TimeSlice>  = BTreeMap::new();
    'month_loop: for (_, month_year) in month_years{ //start time already utc
        //println!("Getting historical data for: {:?}", month_year);
        // Get the historical data for all subscriptions from the server, parse it into TimeSlices.
        let time_slices = match get_historical_data(vec![subscription.clone()], month_year).await {
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
            //println!("Time: {:?}", time);
            if time < from_time {
                continue;
            }

            if time > to_time {
                break  'month_loop;
            }
            data.insert(time, time_slice);
        }
    }
    data
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
    use chrono::{FixedOffset, NaiveDate, TimeZone};
    use crate::apis::vendor::DataVendor;
    use crate::server_connections::{initialize_clients, PlatformMode};
    use crate::standardized_types::enums::MarketType;

    #[tokio::test]
    async fn test_history_base_data() {
        initialize_clients(&PlatformMode::SingleMachine).await.unwrap();
        // Setup the date range
        let from_time = NaiveDate::from_ymd_opt(2023, 03, 19).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let to_time = NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();

        // Convert to FixedOffset for the function calls
        let from_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&from_time).unwrap();
        let to_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&to_time).unwrap();


        let subscription = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex);

        // Call history function
        let result = history(subscription, from_time, to_time).await;

        // Check result
       if let Some(result) = result {
           assert!(result.len() > 20, "Expected more historical data.");
       } else {
           assert!(false, "Expected some historical data.");
       }
    }

    #[tokio::test]
    async fn test_history_many_base_data() {
        initialize_clients(&PlatformMode::SingleMachine).await.unwrap();
        // Setup the date range
        let from_time = NaiveDate::from_ymd_opt(2023, 03, 19).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let to_time = NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();

        // Convert to FixedOffset for the function calls
        let from_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&from_time).unwrap();
        let to_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&to_time).unwrap();


        let subscriptions: Vec<DataSubscription> = vec![
            DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
            DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Ticks(1), BaseDataType::Ticks, MarketType::Forex),
        ];

        // Call history_many function
        let result = history_many(subscriptions, from_time, to_time).await;

        // Check result
        if let Some(result) = result {
            assert!(result.len() > 20, "Expected more historical data.");
        } else {
            assert!(false, "Expected some historical data.");
        }
    }

    #[tokio::test]
    async fn test_history_consolidated_data() {
        initialize_clients(&PlatformMode::SingleMachine).await.unwrap();
        // Setup the date range
        let from_time = NaiveDate::from_ymd_opt(2023, 03, 19).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let to_time = NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();

        // Convert to FixedOffset for the function calls
        let from_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&from_time).unwrap();
        let to_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&to_time).unwrap();


        let subscription = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Seconds(15), BaseDataType::Candles, MarketType::Forex);

        // Call history function
        let result = history(subscription, from_time, to_time).await;

        // Check result
        if let Some(result) = result {
            assert!(result.len() > 20, "Expected more historical data.");
        } else {
            assert!(false, "Expected some historical data.");
        }
    }

    #[tokio::test]
    async fn test_history_many_consolidated_data() {
        initialize_clients(&PlatformMode::SingleMachine).await.unwrap();
        // Setup the date range
        let from_time = NaiveDate::from_ymd_opt(2023, 03, 19).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let to_time = NaiveDate::from_ymd_opt(2023, 03, 30).unwrap().and_hms_opt(0, 0, 0).unwrap();

        // Convert to FixedOffset for the function calls
        let from_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&from_time).unwrap();
        let to_time = FixedOffset::east_opt(0).unwrap().from_local_datetime(&to_time).unwrap();


        let subscriptions: Vec<DataSubscription> = vec![
            DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Seconds(15), BaseDataType::Candles, MarketType::Forex),
            DataSubscription::new("AUD-USD".to_string(), DataVendor::Test, Resolution::Minutes(1), BaseDataType::Candles, MarketType::Forex),
        ];

        // Call history_many function
        let result = history_many(subscriptions, from_time, to_time).await;

        // Check result
        if let Some(result) = result {
            assert!(result.len() > 20, "Expected more historical data.");
            //test the order of the data
            assert!(result.keys().next().unwrap() < result.keys().last().unwrap(), "Expected the data to be ordered by time.");
        } else {
            assert!(false, "Expected some historical data.");
        }
    }
}

