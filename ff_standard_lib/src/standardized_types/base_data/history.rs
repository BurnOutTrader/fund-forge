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
use std::io::Read;
use ahash::AHashMap;
use flate2::bufread::GzDecoder;
use futures::future::join_all;
use tokio::sync::oneshot;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::standardized_types::enums::{StrategyMode, PrimarySubscription};
use crate::standardized_types::market_hours::TradingHours;
use crate::strategies::client_features::request_handler::{send_request, StrategyRequest};
use crate::strategies::client_features::server_connections::SETTINGS_MAP;
use crate::strategies::consolidators::consolidator_enum::ConsolidatorEnum;


// Helper function to process a single compressed payload
async fn process_compressed_payload(
    compressed_data: &[u8],
) -> Result<Vec<BaseDataEnum>, FundForgeError> {
    // Pre-allocate decompressed data with estimated size (3:1 ratio)
    let mut decompressed = Vec::new();
    decompressed.try_reserve(compressed_data.len() * 98)
        .map_err(|e| FundForgeError::ClientSideErrorDebug(
            format!("Failed to allocate memory for decompression: {}", e)
        ))?;

    const MB: usize = 1024 * 1024;
    const BUFFER_SIZE: usize = 13 * MB;
    let mut buffer = vec![0; BUFFER_SIZE];

    let cursor = std::io::Cursor::new(compressed_data);
    let mut decoder = GzDecoder::new(cursor);
    // Read in chunks
    loop {
        match decoder.read(&mut buffer[..]) {
            Ok(0) => break,
            Ok(n) => {
                decompressed.try_reserve(n)
                    .map_err(|e| FundForgeError::ClientSideErrorDebug(
                        format!("Failed to allocate during decompression: {}", e)
                    ))?;
                decompressed.extend_from_slice(&buffer[..n]);
            }
            Err(e) => return Err(FundForgeError::ClientSideErrorDebug(
                format!("Failed to read decompressed data: {}", e)
            )),
        }
    }

    BaseDataEnum::from_array_bytes(&decompressed)
        .map_err(|e| FundForgeError::ClientSideErrorDebug(format!("Failed to parse data: {}", e)))
}

async fn process_payload(
    payload: Vec<Vec<u8>>,
    from_time: DateTime<Utc>,
    to_time: DateTime<Utc>,
) -> Result<BTreeMap<i64, TimeSlice>, FundForgeError> {
    let mut combined_data: BTreeMap<i64, TimeSlice> = BTreeMap::new();

    for compressed_data in payload {
        if let Ok(base_data_vec) = process_compressed_payload(&compressed_data).await {
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
}

pub async fn get_compressed_historical_data(
    subscriptions: Vec<DataSubscription>,
    from_time: DateTime<Utc>,
    to_time: DateTime<Utc>,
) -> Result<BTreeMap<i64, TimeSlice>, FundForgeError> {
    let connections = SETTINGS_MAP.clone();

    if connections.len() <= 2 {
        // Single connection case
        let (tx, rx) = oneshot::channel();
        let request = StrategyRequest::CallBack(
            ConnectionType::Default,
            DataServerRequest::GetCompressedHistoricalData {
                callback_id: 0,
                subscriptions,
                from_time: from_time.to_string(),
                to_time: to_time.to_string(),
            },
            tx
        );

        send_request(request).await;
        //todo make the time out increased for live mode.
        let response = tokio::time::timeout(std::time::Duration::from_secs(15), rx)
            .await
            .expect("Timed out waiting for response after 15 seconds!");

        match response {
            Ok(DataServerResponse::CompressedHistoricalData { payload, .. }) => {
                process_payload(payload, from_time, to_time).await
            },
            Ok(DataServerResponse::Error { error, .. }) => Err(error),
            Ok(_) => Err(FundForgeError::UnknownBlameError("Incorrect callback data received".to_string())),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Failed to receive payload: {}", e)))
        }
    } else {
        // Multi-connection case
        let mut requests_map: AHashMap<ConnectionType, Vec<DataSubscription>> = AHashMap::new();

        for sub in subscriptions {
            let vendor_connection = ConnectionType::Vendor(sub.symbol.data_vendor.clone());
            let entry = if connections.contains_key(&vendor_connection) {
                vendor_connection
            } else {
                ConnectionType::Default
            };
            requests_map.entry(entry).or_default().push(sub);
        }

        let futures: Vec<_> = requests_map
            .into_iter()
            .map(|(connection_type, subs)| {
                let (tx, rx) = oneshot::channel();
                let request = StrategyRequest::CallBack(
                    connection_type,
                    DataServerRequest::GetCompressedHistoricalData {
                        callback_id: 0,
                        subscriptions: subs,
                        from_time: from_time.to_string(),
                        to_time: to_time.to_string(),
                    },
                    tx
                );

                async move {
                    send_request(request).await;
                    let response = tokio::time::timeout(std::time::Duration::from_secs(15), rx)
                        .await
                        .expect("Timed out waiting for response after 15 seconds!");

                    match response {
                        Ok(DataServerResponse::CompressedHistoricalData { payload, .. }) => Ok(payload),
                        Ok(DataServerResponse::Error { error, .. }) => Err(error),
                        Ok(_) => Err(FundForgeError::UnknownBlameError("Incorrect response received at callback".to_string())),
                        Err(e) => Err(FundForgeError::ClientSideErrorDebug(format!("Failed to receive payload: {}", e)))
                    }
                }
            })
            .collect();

        let results = join_all(futures).await;
        let mut combined_data = BTreeMap::new();

        for result in results {
            match result {
                Ok(payload) => {
                    let partial_data = process_payload(payload, from_time, to_time).await?;
                    combined_data.extend(partial_data);
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
    mode: StrategyMode,
    market_hours: Option<TradingHours>,
) -> BTreeMap<DateTime<Utc>, BaseDataEnum> {
    if from_time > to_time {
        panic!("From time cannot be greater than to time");
    }
    let sub_res_type = PrimarySubscription::new(subscription.resolution, subscription.base_data_type);
    let resolutions = subscription.symbol.data_vendor.warm_up_resolutions(subscription.symbol.market_type).await.unwrap();
    if resolutions.contains(&sub_res_type) {
        let data = match get_compressed_historical_data(vec![subscription.clone()], from_time, to_time).await {
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
        let consolidator = ConsolidatorEnum::create_consolidator(subscription, false, market_hours).await;
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
