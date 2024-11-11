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
use std::collections::btree_map::Entry;
use std::io::Read;
use ahash::AHashMap;
use flate2::bufread::GzDecoder;
use futures::future::join_all;
use tokio::sync::oneshot;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::standardized_types::enums::{StrategyMode, SubscriptionResolutionType};
use crate::strategies::client_features::request_handler::{send_request, StrategyRequest};
use crate::strategies::client_features::server_connections::SETTINGS_MAP;
use crate::strategies::consolidators::consolidator_enum::ConsolidatorEnum;

pub async fn get_historical_data(
    subscriptions: Vec<DataSubscription>,
    from_time: DateTime<Utc>,
    to_time: DateTime<Utc>,
) -> Result<BTreeMap<i64, TimeSlice>, FundForgeError> {
    let connections = SETTINGS_MAP.clone();
    if connections.len() <= 2 {
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
            Ok(payload) => {
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
    } else { //todo this is untested.
        // For len > 2, handle potential vendor-specific connections
        let mut requests_map: AHashMap<ConnectionType, Vec<DataSubscription>> = AHashMap::new();

        // Sort subscriptions into appropriate connections
        for sub in subscriptions {
            let vendor_connection = ConnectionType::Vendor(sub.symbol.data_vendor.clone());

            // If vendor connection exists in settings, use it; otherwise use default
            if connections.contains_key(&vendor_connection) {
                requests_map.entry(vendor_connection)
                    .or_insert_with(Vec::new)
                    .push(sub);
            } else {
                requests_map.entry(ConnectionType::Default)
                    .or_insert_with(Vec::new)
                    .push(sub);
            }
        }

        // Create and execute concurrent requests
        let mut futures = Vec::new();
        for (connection_type, subs) in requests_map {
            let (tx, rx) = oneshot::channel();
            let req = DataServerRequest::HistoricalBaseDataRange {
                callback_id: 0,
                subscriptions: subs,
                from_time: from_time.to_string(),
                to_time: to_time.to_string(),
            };
            let request = StrategyRequest::CallBack(
                connection_type,
                req,
                tx
            );

            // Create future for this request
            let future = async move {
                send_request(request).await;
                match rx.await {
                    Ok(response) => match response {
                        DataServerResponse::HistoricalBaseData { payload, .. } => Ok(payload),
                        DataServerResponse::Error { error, .. } => Err(error),
                        _ => Err(FundForgeError::UnknownBlameError("Incorrect response received at callback".to_string()))
                    },
                    Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Failed to receive payload: {}", e)))
                }
            };

            futures.push(future);
        }

        // Wait for all requests to complete
        let results = join_all(futures).await;

        // Combine BTreeMap results, merging TimeSlices for same timestamps
        let mut combined_data: BTreeMap<i64, TimeSlice> = BTreeMap::new();
        for result in results {
            match result {
                Ok(payload) => {
                    // Merge each payload's data into the combined map
                    for (timestamp, time_slice) in payload {
                        match combined_data.entry(timestamp) {
                            Entry::Vacant(entry) => {
                                entry.insert(time_slice);
                            },
                            Entry::Occupied(mut entry) => {
                                // Merge the new TimeSlice with existing one
                                entry.get_mut().merge(time_slice);
                            }
                        }
                    }
                },
                Err(e) => return Err(e),
            }
        }

        Ok(combined_data)
    }
}

pub async fn get_compressed_historical_data (
    subscriptions: Vec<DataSubscription>,
    from_time: DateTime<Utc>,
    to_time: DateTime<Utc>,
) -> Result<BTreeMap<i64, TimeSlice>, FundForgeError> {
    let connections = SETTINGS_MAP.clone();
    if connections.len() <= 2 {
        let req = DataServerRequest::GetCompressedHistoricalData {
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
            Ok(payload) => {
                match payload {
                    DataServerResponse::CompressedHistoricalData { payload, .. } => {
                        let mut combined_data: BTreeMap<i64, TimeSlice> = BTreeMap::new();

                        // Process each compressed file's data
                        for compressed_data in payload {
                            // Create a cursor for the compressed data and wrap it in a BufReader
                            let cursor = std::io::Cursor::new(&compressed_data);
                            let buf_reader = std::io::BufReader::new(cursor);
                            let mut decoder = GzDecoder::new(buf_reader);

                            // Use a BufReader for efficient reading of decompressed data
                            let mut buf_decompressed = std::io::BufReader::new(&mut decoder);
                            let mut decompressed = Vec::new();
                            match buf_decompressed.read_to_end(&mut decompressed) {
                                Ok(_) => {},
                                Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to read decompressed data: {}", e)))
                            }

                            // Parse the decompressed data into BaseDataEnum vec
                            if let Ok(base_data_vec) = BaseDataEnum::from_array_bytes(&decompressed) {
                                // Filter data within the requested time range and add to combined_data
                                for data in base_data_vec {
                                    if data.time_closed_utc() >= from_time && data.time_closed_utc() <= to_time {
                                        let timestamp = data.time_closed_utc().timestamp_nanos_opt()
                                            .ok_or_else(|| FundForgeError::ClientSideErrorDebug(
                                                "Failed to convert timestamp to nanos".to_string()
                                            ))?;

                                        combined_data
                                            .entry(timestamp)
                                            .or_insert_with(TimeSlice::new)
                                            .add(data);
                                    }
                                }
                            }
                        }
                        Ok(combined_data)
                    },
                    DataServerResponse::Error { error, .. } => {
                        Err(error)
                    },
                    _ => Err(FundForgeError::UnknownBlameError("Incorrect callback data received".to_string()))
                }
            }
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Failed to receive payload: {}", e)))
        }
    } else {
        // For len > 2, handle potential vendor-specific connections
        let mut requests_map: AHashMap<ConnectionType, Vec<DataSubscription>> = AHashMap::new();

        // Sort subscriptions into appropriate connections
        for sub in subscriptions {
            let vendor_connection = ConnectionType::Vendor(sub.symbol.data_vendor.clone());

            // If vendor connection exists in settings, use it; otherwise use default
            if connections.contains_key(&vendor_connection) {
                requests_map.entry(vendor_connection)
                    .or_insert_with(Vec::new)
                    .push(sub);
            } else {
                requests_map.entry(ConnectionType::Default)
                    .or_insert_with(Vec::new)
                    .push(sub);
            }
        }

        // Create and execute concurrent requests
        let mut futures = Vec::new();
        for (connection_type, subs) in requests_map {
            let (tx, rx) = oneshot::channel();
            let req = DataServerRequest::GetCompressedHistoricalData {
                callback_id: 0,
                subscriptions: subs,
                from_time: from_time.to_string(),
                to_time: to_time.to_string(),
            };
            let request = StrategyRequest::CallBack(
                connection_type,
                req,
                tx
            );

            // Create future for this request
            let future = async move {
                send_request(request).await;
                match rx.await {
                    Ok(response) => match response {
                        DataServerResponse::CompressedHistoricalData { payload, .. } => Ok(payload),
                        DataServerResponse::Error { error, .. } => Err(error),
                        _ => Err(FundForgeError::UnknownBlameError("Incorrect response received at callback".to_string()))
                    },
                    Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Failed to receive payload: {}", e)))
                }
            };

            futures.push(future);
        }

        // Wait for all requests to complete
        let results = join_all(futures).await;

        // Combine results from all connections
        let mut combined_data: BTreeMap<i64, TimeSlice> = BTreeMap::new();
        for result in results {
            match result {
                Ok(payload) => {
                    // Process each connection's compressed files
                    for compressed_data in payload {
                        // Create a cursor for the compressed data and wrap it in a BufReader
                        let cursor = std::io::Cursor::new(&compressed_data);
                        let buf_reader = std::io::BufReader::new(cursor);
                        let mut decoder = GzDecoder::new(buf_reader);

                        // Use a BufReader for efficient reading of decompressed data
                        let mut buf_decompressed = std::io::BufReader::new(&mut decoder);
                        let mut decompressed = Vec::new();
                        match buf_decompressed.read_to_end(&mut decompressed) {
                            Ok(_) => {},
                            Err(e) => return Err(FundForgeError::ClientSideErrorDebug(format!("Failed to read decompressed data: {}", e)))
                        }

                        // Parse the decompressed data
                        if let Ok(base_data_vec) = BaseDataEnum::from_array_bytes(&decompressed) {
                            // Filter and add data within the time range
                            for data in base_data_vec {
                                if data.time_closed_utc() >= from_time && data.time_closed_utc() <= to_time {
                                    let timestamp = data.time_closed_utc().timestamp_nanos_opt()
                                        .ok_or_else(|| FundForgeError::ClientSideErrorDebug(
                                            "Failed to convert timestamp to nanos".to_string()
                                        ))?;

                                    combined_data
                                        .entry(timestamp)
                                        .or_insert_with(TimeSlice::new)
                                        .add(data);
                                }
                            }
                        }
                    }
                },
                Err(e) => return Err(e),
            }
        }

        Ok(combined_data)
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
    let resolutions = subscription.symbol.data_vendor.warm_up_resolutions(subscription.symbol.market_type).await.unwrap();
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
