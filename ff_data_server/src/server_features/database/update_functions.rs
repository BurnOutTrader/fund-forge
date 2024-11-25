use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use async_std::task::sleep;
use chrono::{DateTime, NaiveDate, Utc};
use dashmap::DashMap;
use indicatif::ProgressBar;
use serde::{Deserialize, Deserializer};
use strum::IntoEnumIterator;
use tokio::sync::Semaphore;
use tokio::task;
use tokio::task::JoinHandle;
use ff_standard_lib::messages::data_server_messaging::FundForgeError;
use ff_standard_lib::product_maps::rithmic::maps::get_exchange_by_symbol_name;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::MarketType;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{Symbol, SymbolName};
use crate::oanda_api::api_client::{OANDA_CLIENT, OANDA_IS_CONNECTED};
use crate::rithmic_api::api_client::{get_rithmic_market_data_system, RITHMIC_CLIENTS, RITHMIC_DATA_IS_CONNECTED};
use crate::server_features::database::hybrid_storage::{HybridStorage, MULTIBAR};
use crate::server_features::server_side_datavendor::VendorApiResponse;
use crate::{get_data_folder, subscribe_server_shutdown};


#[derive(Deserialize)]
struct DownloadSymbols {
    pub(crate) symbols: Vec<DownloadConfig>,
}

#[derive(Deserialize, Debug)]
pub struct DownloadConfig {
    pub symbol_name: SymbolName,
    pub base_data_type: BaseDataType,
    pub start_date: NaiveDate,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub resolution: Resolution,
}

fn deserialize_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}


impl HybridStorage {
    pub fn run_update_schedule(self: Arc<Self>) {
        let mut shutdown_receiver = subscribe_server_shutdown();

        tokio::spawn(async move {
            // Main interval for regular updates (defined by update_seconds)
            let mut interval = tokio::time::interval(Duration::from_secs(self.update_seconds));

            // New: Additional interval specifically for cleaning up finished tasks
            let mut cleanup_interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            loop {
                tokio::select! {
                // Handle shutdown signal
                _ = shutdown_receiver.recv() => {
                    println!("Received shutdown signal, stopping update schedule");
                    // Actively abort all running tasks
                    for task in self.download_tasks.iter() {
                        task.value().abort();
                    }
                    self.download_tasks.clear();
                    MULTIBAR.clear().ok();
                    break;
                }

                // New: Periodic cleanup every 5 minutes
                _ = cleanup_interval.tick() => {
                    // Remove any completed tasks from the map
                    self.download_tasks.retain(|_, task| !task.is_finished());
                    if self.download_tasks.is_empty() {
                        MULTIBAR.clear().ok();
                    }
                }

                // Regular update interval
                _ = interval.tick() => {
                    // Wait for any existing tasks to complete before starting new ones
                    if !self.download_tasks.is_empty() {
                        self.download_tasks.retain(|_, task| !task.is_finished());

                        if self.download_semaphore.available_permits() == 0 {
                            continue;
                        }
                    }

                    // Run forward update
                    if let Err(e) = HybridStorage::update_data(self.clone(), false).await {
                        eprintln!("Forward update failed: {}", e);
                    }

                    // Wait for forward tasks to complete before starting backward tasks
                    self.download_tasks.retain(|_, task| !task.is_finished());
                    if !self.download_tasks.is_empty() {
                        self.download_tasks.retain(|_, task| !task.is_finished());
                    }

                    //todo Run backward update after forward completes, these need to be modified to use a dif fn and download from earliest time backwards
               /*     if let Err(e) = HybridStorage::update_data(self.clone(), true).await {
                        eprintln!("Backward update failed: {}", e);
                    }*/
                }
            }
            }
        });
    }

    pub async fn pre_subscribe_updates(&self, symbol: Symbol, resolution: Resolution, base_data_type: BaseDataType) {
        let client: Arc<dyn VendorApiResponse> = match symbol.data_vendor {
            DataVendor::Rithmic if RITHMIC_DATA_IS_CONNECTED.load(Ordering::SeqCst) => {
                match get_rithmic_market_data_system().and_then(|sys| RITHMIC_CLIENTS.get(&sys)) {
                    Some(client) => client.clone(),
                    None => return,
                }
            }
            DataVendor::Oanda if OANDA_IS_CONNECTED.load(Ordering::SeqCst)=> {
                match OANDA_CLIENT.get() {
                    Some(client) => client.clone(),
                    None => return,
                }
            }
            _ => return,
        };

        let start_time = match self.get_latest_data_time(&symbol, &resolution, &base_data_type).await {
            Ok(Some(date)) => date,
            Err(_) | Ok(None) => {
                let path = get_data_folder()
                    .join("credentials")
                    .join(format!("{}_credentials", symbol.data_vendor.to_string().to_lowercase()))
                    .join("download_list.toml");

                if path.exists() {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(_) => return, // Exit the entire function
                    };

                    let symbol_configs = match toml::from_str::<DownloadSymbols>(&content) {
                        Ok(symbol_object) => symbol_object.symbols,
                        Err(_) => return, // Exit the entire function
                    };

                    let symbol_config = match symbol_configs.iter().find(|s| {
                        s.symbol_name == symbol.name && s.resolution == resolution && s.base_data_type == base_data_type
                    }) {
                        Some(config) => config,
                        None => return, // Exit the entire function
                    };

                    // Return the correct time if we have one
                    DateTime::<Utc>::from_naive_utc_and_offset(
                        symbol_config.start_date.and_hms_opt(0, 0, 0).unwrap(),
                        Utc,
                    )
                } else {
                    return; // Exit the entire function if the path does not exist
                }
            }
        };
        let key = (symbol.name.clone(), base_data_type.clone(), resolution.clone());

        let mut was_downloading = false;
        while self.download_tasks.contains_key(&key) {
            was_downloading = true;
            sleep(Duration::from_secs(1)).await;
        }
        if was_downloading {
            return;
        }

        let symbol_pb = MULTIBAR.add(ProgressBar::new(1));
        symbol_pb.set_prefix(format!("{}", symbol.name));

        let download_tasks = self.download_tasks.clone();
        let key_clone = key.clone();
        {
            self.download_tasks.insert(key.clone(), task::spawn(async move {
                match client.update_historical_data(symbol.clone(), base_data_type, resolution, start_time, Utc::now() + Duration::from_secs(15), false, symbol_pb, false).await {
                    Ok(_) => {
                        download_tasks.remove(&key_clone);
                    },
                    Err(_) => {
                        download_tasks.remove(&key_clone);
                    }
                }
            }));
        }

        while self.download_tasks.contains_key(&key) {
            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn update_data(self: Arc<Self>, from_back: bool) -> Result<(), FundForgeError> {
        let options = self.options.clone();
        // Create a semaphore to limit concurrent downloads
        let semaphore = self.download_semaphore.clone();

        for vendor in DataVendor::iter() {
            match vendor {
                DataVendor::Rithmic if !RITHMIC_DATA_IS_CONNECTED.load(Ordering::SeqCst) => {
                    continue
                },
                DataVendor::Oanda if !OANDA_IS_CONNECTED.load(Ordering::SeqCst) => {
                    continue
                },
                DataVendor::DataBento | DataVendor::Bitget => {
                    continue
                },
                _ => (),
            }
            // choose the path based on the vendor
            let path =  options.data_folder.clone().join("credentials").join(format!("{}_credentials", vendor.to_string().to_lowercase())).join("download_list.toml");
            //eprintln!("Path: {:?}", path);

            if path.exists() {
                let content = match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(e) => {
                        return Err(FundForgeError::ServerErrorDebug(e.to_string()));
                    }
                };

                let symbol_configs = match toml::from_str::<DownloadSymbols>(&content) {
                    Ok(symbol_object) => symbol_object.symbols,
                    Err(e) => {
                        return Err(FundForgeError::ServerErrorDebug(e.to_string()));
                    }
                };

                if !symbol_configs.is_empty() {
                    for symbol_config in symbol_configs {
                        if self.download_tasks.contains_key(&(symbol_config.symbol_name.clone(), symbol_config.base_data_type, symbol_config.resolution)) {
                            continue;
                        }
                        //eprintln!("Symbol: {:?}", symbol_config);
                        let market_type = match vendor {
                            DataVendor::Oanda => {
                                if let Some(client) = OANDA_CLIENT.get() {
                                    if let Some(instrument) = client.instruments_map.get(&symbol_config.symbol_name) {
                                        instrument.value().market_type
                                    } else {
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            },
                            DataVendor::Rithmic => {
                                match get_exchange_by_symbol_name(&symbol_config.symbol_name) {
                                    Some(exchange) => MarketType::Futures(exchange),
                                    None => {
                                        continue
                                    },
                                }
                            }
                            _ => {
                                continue
                            }
                        };

                        let symbol = Symbol::new(symbol_config.symbol_name.clone(), vendor.clone(), market_type);

                        let start_time = match from_back {
                            true => {
                                DateTime::<Utc>::from_naive_utc_and_offset(
                                    symbol_config.start_date.and_hms_opt(0, 0, 0).unwrap(),
                                    Utc
                                )
                            },
                            false => {
                                match self.get_latest_data_time(&symbol, &symbol_config.resolution, &symbol_config.base_data_type).await {
                                    Ok(Some(date)) => date,
                                    Err(_) | Ok(None) => {
                                        DateTime::<Utc>::from_naive_utc_and_offset(
                                            symbol_config.start_date.and_hms_opt(0, 0, 0).unwrap(),
                                            Utc
                                        )
                                    }
                                }
                            }
                        };

                        let end_time = if !from_back {
                            Utc::now()
                        } else {
                            let earliest = match self.get_earliest_data_time(&symbol, &symbol_config.resolution, &symbol_config.base_data_type).await {
                                Ok(Some(date)) if date > start_time => Some(date), // If we have data and it's after our target start time
                                _ => continue
                            };
                            match earliest {
                                Some(time) => time,
                                None => {
                                    continue
                                },
                            }
                        };

                        // Verify chronological order for backwards downloads
                        if end_time <= start_time {
                            continue;
                        }


                        if from_back == true {
                            let latest_date = match self.get_latest_data_time(&symbol, &symbol_config.resolution, &symbol_config.base_data_type).await {
                                Ok(Some(date)) => date,
                                Err(_) | Ok(None) => {
                                    continue //skip move start date back if we have no existing data
                                }
                            };
                            if from_back && start_time >= end_time - chrono::Duration::days(3) {
                                continue;
                            }
                            // Only skip if we're moving backwards and we've already reached our target or we have updated data to the present
                            if start_time >= end_time - chrono::Duration::days(3) || end_time.date_naive() > Utc::now().date_naive() - chrono::Duration::days(3) || latest_date < Utc::now() - chrono::Duration::days(3)  {
                                continue;
                            }
                        }


                        let semaphore = semaphore.clone();
                        let download_tasks = self.download_tasks.clone();
                        // Directly spawn the update_symbol task
                        // Create and configure symbol progress bar with an initial length

                        HybridStorage::update_symbol(
                            download_tasks.clone(),
                            semaphore,
                            symbol.clone(),
                            symbol_config.resolution,
                            symbol_config.base_data_type.clone(),
                            start_time,
                            end_time,
                            from_back,
                            true
                        ).await;
                    }
                }
            }
        }
        Ok(())
    }


    async fn update_symbol(
        download_tasks: Arc<DashMap<(SymbolName, BaseDataType, Resolution), JoinHandle<()>>>,
        download_semaphore: Arc<Semaphore>,
        symbol: Symbol,
        resolution: Resolution,
        base_data_type: BaseDataType,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        from_back: bool,
        is_bulk_download: bool
    ) {
        let key = (symbol.name.clone(), base_data_type.clone(), resolution.clone());

        // Check if there's already a task running for this key
        if let Some(task) = download_tasks.get(&key) {
            if task.is_finished() {
                download_tasks.remove(&key);
            } else {
                return;
            }
        }

        // Get the client before attempting to acquire the semaphore
        let client: Arc<dyn VendorApiResponse> = match symbol.data_vendor {
            DataVendor::Rithmic if RITHMIC_DATA_IS_CONNECTED.load(Ordering::SeqCst) => {
                match get_rithmic_market_data_system().and_then(|sys| RITHMIC_CLIENTS.get(&sys)) {
                    Some(client) => client.clone(),
                    None => return,
                }
            }
            DataVendor::Oanda if OANDA_IS_CONNECTED.load(Ordering::SeqCst)=> {
                match OANDA_CLIENT.get() {
                    Some(client) => client.clone(),
                    None => return,
                }
            }
            _ => return,
        };

        // Now spawn the real task
        let download_tasks_clone = download_tasks.clone();
        let key_clone = key.clone();
        let task = task::spawn(async move {
            let symbol_pb = MULTIBAR.add(ProgressBar::new(1));
            // Acquire the permit inside the spawned task
            let _permit = match download_semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    download_tasks_clone.remove(&key_clone);
                    return;
                }
            };

            let prefix = match Utc::now().date_naive() == to.date_naive() {
                true => "Moving Data End Time Forwards",
                false => "Moving Data Start Time Backwards",
            };
            symbol_pb.set_prefix(format!("{}: {}", prefix, symbol.name));

            match client.update_historical_data(symbol.clone(), base_data_type, resolution, from, to, from_back, symbol_pb, is_bulk_download).await {
                Ok(_) => {},
                Err(_) => {}
            }

            // Remove from active tasks
            download_tasks_clone.remove(&key_clone);
            // permit is automatically dropped here
        });

        // Replace the dummy task with the real one
        download_tasks.insert(key, task);
    }

}