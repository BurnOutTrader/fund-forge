use std::cmp::min;
use strum::IntoEnumIterator;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::fs;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use std::path::{Path, PathBuf};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::str::FromStr;
use std::sync::{Arc};
use std::sync::atomic::{Ordering};
use std::time::Duration;
use async_std::task::sleep;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use dashmap::DashMap;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use futures::future;
use indicatif::{MultiProgress, ProgressBar};
use lazy_static::lazy_static;
use memmap2::{Mmap};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Deserializer};
use tokio::sync::{OnceCell, Semaphore};
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::interval;
use ff_standard_lib::messages::data_server_messaging::{FundForgeError};
use ff_standard_lib::product_maps::oanda::maps::OANDA_SYMBOL_INFO;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::{MarketType, OrderSide};
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use crate::oanda_api::api_client::{OANDA_CLIENT, OANDA_IS_CONNECTED};
use crate::rithmic_api::api_client::{get_rithmic_market_data_system, RITHMIC_CLIENTS, RITHMIC_DATA_IS_CONNECTED};
use ff_standard_lib::product_maps::rithmic::maps::get_exchange_by_symbol_name;
use ff_standard_lib::standardized_types::accounts::Currency;
use crate::server_features::server_side_datavendor::VendorApiResponse;
use crate::{get_data_folder, subscribe_server_shutdown, ServerLaunchOptions};

pub static DATA_STORAGE: OnceCell<Arc<HybridStorage>> = OnceCell::const_new();

lazy_static!(
    static ref MULTIBAR: MultiProgress = MultiProgress::new();
);

#[allow(unused)]
pub struct HybridStorage {
    base_path: PathBuf,
    mmap_cache: Arc<DashMap<String, Arc<Mmap>>>,
    cache_last_accessed: Arc<DashMap<String, DateTime<Utc>>>,
    clear_cache_duration: Duration,
    file_locks: Arc<DashMap<String, Semaphore>>,
    download_tasks: Arc<DashMap<(SymbolName, BaseDataType, Resolution), JoinHandle<()>>>,
    options: ServerLaunchOptions,
    download_semaphore: Arc<Semaphore>,
    update_seconds: u64,
}

impl HybridStorage {
    pub fn new(clear_cache_duration: Duration, options: ServerLaunchOptions, max_concurrent_downloads: usize, update_seconds: u64) -> Self {
        let max_concurrent_downloads= min(max_concurrent_downloads, 40);
        let storage = Self {
            base_path: options.data_folder.clone().join("historical"),
            mmap_cache: Arc::new(DashMap::new()),
            cache_last_accessed: Arc::new(DashMap::new()),
            clear_cache_duration,
            file_locks: Default::default(),
            download_tasks:Default::default(),
            options,
            download_semaphore: Arc::new(Semaphore::new(max_concurrent_downloads)),
            update_seconds
        };

        storage
    }


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
            was_downloading = true;
            sleep(Duration::from_secs(1)).await;
        }
    }

    // Helper function to decompress and deserialize data from a mmap
    async fn decompress_and_deserialize(&self, mmap: &[u8]) -> io::Result<Vec<BaseDataEnum>> {
        // Create a decoder for the compressed data
        let mut decoder = GzDecoder::new(mmap);
        let mut decompressed = Vec::new();

        // Decompress the data
        decoder.read_to_end(&mut decompressed).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Decompression error: {}", e))
        })?;

        // Deserialize the decompressed data
        BaseDataEnum::from_array_bytes(&decompressed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Deserialization error: {}", e)))
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
                DataVendor::Test | DataVendor::Bitget => {
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

    pub(crate) fn start_cache_management(self: Arc<Self>) {
        let mmap_cache = Arc::clone(&self.mmap_cache);
        let cache_last_accessed = Arc::clone(&self.cache_last_accessed);
        let clear_cache_duration = self.clear_cache_duration;
        let file_locks = self.file_locks.clone();
        task::spawn(async move {
            let mut interval = interval(clear_cache_duration);

            loop {
                interval.tick().await;

                let now = Utc::now();
                let expiration_duration = chrono::Duration::from_std(clear_cache_duration).unwrap();

                // Collect keys that should be removed
                let mut keys_to_remove = Vec::new();

                // Iterate over `cache_last_accessed` to find expired entries
                for entry in cache_last_accessed.iter() {
                    let path = entry.key();
                    let last_access = entry.value();

                    // Check if the entry has expired
                    if now.signed_duration_since(*last_access) > expiration_duration {
                        // Save the mmap to disk if it was updated
                        // Mark this path for removal
                        keys_to_remove.push(path.clone());
                    }
                }

                // Remove expired entries from all caches
                for path in keys_to_remove {
                    mmap_cache.remove(&path);
                    if let Some((_, mmap)) = mmap_cache.remove(&path) {
                        drop(mmap); // Explicitly drop the mmap
                    }
                    cache_last_accessed.remove(&path);
                    let mut remove_semaphore = false;
                    if let Some(file_semaphore) = file_locks.get(&path) {
                        if file_semaphore.available_permits() == 1 {
                            remove_semaphore = true;
                        }
                    }
                    if remove_semaphore {
                        file_locks.remove(&path);
                    }
                }
            }
        });
    }

    fn get_base_path(&self, symbol: &Symbol, resolution: &Resolution, data_type: &BaseDataType, is_saving: bool) -> PathBuf {
        let base_path = self.base_path
            .join(symbol.data_vendor.to_string())
            .join(symbol.market_type.to_string())
            .join(symbol.name.to_string())
            .join(resolution.to_string())
            .join(data_type.to_string());

        //println!("Base Path: {:?}", base_path);

        if is_saving && !base_path.exists() {
            fs::create_dir_all(&base_path).unwrap();
        }

        base_path
    }

    fn get_file_path(&self, symbol: &Symbol, resolution: &Resolution, data_type: &BaseDataType, date: &DateTime<Utc>, is_saving: bool) -> PathBuf {
        let base_path = self.get_base_path(symbol, resolution, data_type, is_saving);
        let path = base_path
            .join(format!("{:04}", date.year()))
            .join(format!("{:02}", date.month()));
        if !path.exists() && is_saving {
            let _ = create_dir_all(&path);
        }
        path.join(format!("{:04}{:02}{:02}.bin", date.year(), date.month(), date.day()))
    }

    pub async fn save_data(&self, data: &BaseDataEnum) -> io::Result<()> {
        if !data.is_closed() {
            return Ok(());
        }

        let file_path = self.get_file_path(
            data.symbol(),
            &data.resolution(),
            &data.base_data_type(),
            &data.time_closed_utc(),
            true
        );
        self.save_data_to_file(&file_path, &[data.clone()], false).await
    }

    pub async fn save_data_bulk(&self, data: Vec<BaseDataEnum>, is_bulk_download: bool) -> io::Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let mut grouped_data: HashMap<(Symbol, Resolution, BaseDataType, DateTime<Utc>), Vec<BaseDataEnum>> = HashMap::new();

        for d in data {
            if !d.is_closed() {
                continue;
            }
            let key = (
                d.symbol().clone(),
                d.resolution(),
                d.base_data_type(),
                d.time_closed_utc().date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap()
            );
            grouped_data.entry(key).or_insert_with(Vec::new).push(d);
        }

        //println!("Grouped data into {} files", grouped_data.len());

        for ((symbol, resolution, data_type, date), group) in grouped_data {
            let file_path = self.get_file_path(&symbol, &resolution, &data_type, &date, true);
            //println!("Saving {} data points to file: {:?}", group.len(), file_path);
            self.save_data_to_file(&file_path, &group, is_bulk_download).await?;
        }

        Ok(())
    }

    pub async fn get_earliest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        let base_path = self.get_base_path(symbol, resolution, data_type, false);
        if !base_path.exists() {
            return Ok(None);
        }

        let mut earliest_time: Option<DateTime<Utc>> = None;

        // Get years in ascending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|s| s.parse::<i32>().ok())
                    .map(|year| (year, e.path()))
            })
            .collect();
        years.sort_unstable_by(|a, b| a.0.cmp(&b.0));  // Sort ascending

        for (_year, year_path) in years {
            // Get months in ascending order
            let mut months: Vec<_> = fs::read_dir(year_path)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|month| (month, e.path()))
                })
                .collect();
            months.sort_unstable_by(|a, b| a.0.cmp(&b.0));  // Sort ascending

            for (_month, month_path) in months {
                // Get days in ascending order
                let mut days: Vec<_> = fs::read_dir(month_path)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
                    .collect();
                days.sort_by_key(|e| e.path());

                for day in days {
                    let file_path = day.path();
                    let mut file = match File::open(&file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            eprintln!("Error opening file {}: {}", file_path.display(), e);
                            continue;
                        }
                    };

                    let mut compressed_data = Vec::new();
                    if let Err(e) = file.read_to_end(&mut compressed_data) {
                        eprintln!("Error reading file {}: {}", file_path.display(), e);
                        continue;
                    }

                    // Decompress
                    let mut decoder = GzDecoder::new(&compressed_data[..]);
                    let mut decompressed = Vec::new();
                    match decoder.read_to_end(&mut decompressed) {
                        Ok(_) => {
                            match BaseDataEnum::from_array_bytes(&decompressed) {
                                Ok(day_data) => {
                                    if let Some(time) = day_data.into_iter()
                                        .map(|d| d.time_closed_utc())
                                        .min() {
                                        match earliest_time {
                                            None => earliest_time = Some(time),
                                            Some(current_earliest) if time < current_earliest => {
                                                earliest_time = Some(time)
                                            }
                                            _ => {}
                                        }
                                        // Found valid data, no need to check more files
                                        return Ok(earliest_time);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error deserializing data from {}: {}", file_path.display(), e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error decompressing data from {}: {}", file_path.display(), e);
                            continue;
                        }
                    }
                }
            }
        }

        Ok(earliest_time)
    }

    async fn get_or_create_mmap(&self, file_path: &Path) -> io::Result<Arc<Mmap>> {
        let path_str = file_path.to_string_lossy().to_string();

        if let Some(mmap) = self.mmap_cache.get(&path_str) {
            self.cache_last_accessed.insert(path_str.clone(), Utc::now());
            return Ok(Arc::clone(mmap.value()));
        }

        // Get oldest file outside of any locks
        {
            let oldest_path = if self.mmap_cache.len() >= 200 {
                self.cache_last_accessed
                    .iter()
                    .min_by_key(|entry| entry.value().clone())
                    .map(|entry| entry.key().clone())
            } else {
                None
            };

            // Remove oldest file if needed
            if let Some(oldest_path) = oldest_path {
                // Get (don't remove) the semaphore first
                if let Some(semaphore) = self.file_locks.get(&oldest_path) {
                    let permit = semaphore.value().acquire().await.map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::Other,
                            format!("Error acquiring lock for cache cleanup: {}", e)
                        )
                    })?;

                    // Now safely remove from caches
                    if let Some((_, mmap)) = self.mmap_cache.remove(&file_path.to_string_lossy().to_string()) {
                        drop(mmap); // Explicitly drop the mmap
                    }
                    self.cache_last_accessed.remove(&oldest_path);
                    drop(permit);
                }
                // Finally remove the semaphore
                self.file_locks.remove(&oldest_path);
            }
        }

        let mut file = File::open(file_path)?;
        let mut compressed_data = Vec::new();
        file.read_to_end(&mut compressed_data)?;

        // Decompress
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        // Create temporary file for mmap
        let temp_path = file_path.with_extension("tmp");
        {
            let mut temp_file = File::create(&temp_path)?;
            temp_file.write_all(&decompressed)?;
            temp_file.sync_all()?;
        }

        // Now open the temp file for mmap
        let temp_file = File::open(&temp_path)?;
        let mmap = Arc::new(unsafe { Mmap::map(&temp_file)? });
        self.mmap_cache.insert(path_str.clone(), Arc::clone(&mmap));
        self.cache_last_accessed.insert(path_str, Utc::now());

        // Clean up temp file
        drop(temp_file);
        std::fs::remove_file(&temp_path)?;

        Ok(mmap)
    }


    /// This first updates the file on disk, then the file in memory is replaced with the new file, therefore we do not have saftey issues.
    async fn save_data_to_file(&self, file_path: &Path, new_data: &[BaseDataEnum], is_bulk_download: bool) -> io::Result<()> {
        let semaphore = self.file_locks.entry(file_path.to_str().unwrap().to_string()).or_insert(Semaphore::new(1));
        let _permit = match semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                panic!("Thread Tripwire, Error acquiring save permit: {}", e)
            }
        };

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(file_path)?;

        // Read and decompress existing data
        let mut compressed_data = Vec::new();
        file.read_to_end(&mut compressed_data)?;

        let existing_data = if !compressed_data.is_empty() {
            let mut decoder = GzDecoder::new(&compressed_data[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            BaseDataEnum::from_array_bytes(&decompressed)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        } else {
            Vec::new()
        };

        let mut data_map: BTreeMap<DateTime<Utc>, BaseDataEnum> = existing_data
            .into_iter()
            .map(|d| (d.time_closed_utc(), d))
            .collect();

        for data_point in new_data {
            data_map.insert(data_point.time_closed_utc(), data_point.clone());
        }

        let all_data: Vec<BaseDataEnum> = data_map.into_values().collect();

        // Serialize with rkyv
        let bytes = BaseDataEnum::vec_to_bytes(all_data);

        // Compress the serialized data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&bytes)?;
        let compressed = encoder.finish()?;

        // Write to file
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;

        match file.write_all(&compressed) {
            Ok(_) => {
                file.sync_all()?;
            },
            Err(e) => {
                drop(file);
                if let Err(remove_err) = std::fs::remove_file(&file_path) {
                    eprintln!("Failed to remove corrupt file {}: {}", file_path.display(), remove_err);
                }
                return Err(e);
            }
        }

        if !is_bulk_download {
            let mmap = unsafe { Mmap::map(&file)? };
            self.mmap_cache.insert(file_path.to_string_lossy().to_string(), Arc::new(mmap));
        } else {
            if let Some((_, mmap)) = self.mmap_cache.remove(&file_path.to_string_lossy().to_string()) {
                drop(mmap);
            }
        }

        Ok(())
    }

    // Updated get_data_range function
    pub async fn get_data_range(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<BaseDataEnum>, FundForgeError> {
        let mut all_data = Vec::new();
        let base_path = self.get_base_path(symbol, resolution, data_type, false);
        if !base_path.exists() {
            return Ok(Vec::new());
        }

        let start_year = start.year();
        let end_year = end.year();

        for year in start_year..=end_year {
            let year_path = base_path.join(format!("{:04}", year));
            if !year_path.exists() { continue; }

            let start_month = if year == start_year { start.month() } else { 1 };
            let end_month = if year == end_year { end.month() } else { 12 };

            for month in start_month..=end_month {
                let month_path = year_path.join(format!("{:02}", month));
                if !month_path.exists() { continue; }

                let current_date = if year == start_year && month == start.month() {
                    start.date_naive()
                } else {
                    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
                };

                let month_end = if year == end_year && month == end.month() {
                    end.date_naive()
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1)
                        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                        .pred_opt()
                        .unwrap()
                };

                let mut current_date = current_date;
                while current_date <= month_end {
                    let file_path = month_path.join(format!("{:04}{:02}{:02}.bin",
                                                            year,
                                                            month,
                                                            current_date.day()
                    ));

                    if file_path.exists() {
                        let mut file = match File::open(&file_path) {
                            Ok(file) => file,
                            Err(_) => {
                                current_date = current_date.succ_opt().unwrap_or(month_end);
                                continue;
                            }
                        };

                        let mut compressed_data = Vec::new();
                        if let Ok(_) = file.read_to_end(&mut compressed_data) {
                            let mut decoder = GzDecoder::new(&compressed_data[..]);
                            let mut decompressed = Vec::new();

                            if let Ok(_) = decoder.read_to_end(&mut decompressed) {
                                match BaseDataEnum::from_array_bytes(&decompressed) {
                                    Ok(mut day_data) => {
                                        // Filter data within the time range
                                        day_data.retain(|d| {
                                            let time = d.time_closed_utc();
                                            time >= start && time <= end
                                        });
                                        all_data.extend(day_data);
                                    }
                                    Err(e) => {
                                        eprintln!("Error deserializing data from {}: {}", file_path.display(), e);
                                    }
                                }
                            }
                        }
                    }

                    current_date = match current_date.succ_opt() {
                        Some(date) => date,
                        None => break,
                    };
                }
            }
        }

        // Sort the data by timestamp before returning
        all_data.sort_by_key(|d| d.time_closed_utc());
        Ok(all_data)
    }

    // Helper function to consistently handle file operations
    async fn read_and_deserialize_file(file_path: &Path) -> Result<Vec<BaseDataEnum>, std::io::Error> {
        let mut file = File::open(file_path)?;
        let mut compressed_data = Vec::new();
        file.read_to_end(&mut compressed_data)?;

        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        BaseDataEnum::from_array_bytes(&decompressed)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub async fn get_files_in_range(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PathBuf>, FundForgeError> {
        let mut file_paths = Vec::new();
        let base_path = self.get_base_path(symbol, resolution, data_type, false);

        let start_year = start.year();
        let end_year = end.year();

        for year in start_year..=end_year {
            let year_path = base_path.join(format!("{:04}", year));
            if !year_path.exists() { continue; }

            let start_month = if year == start_year { start.month() } else { 1 };
            let end_month = if year == end_year { end.month() } else { 12 };

            for month in start_month..=end_month {
                let month_path = year_path.join(format!("{:02}", month));
                if !month_path.exists() { continue; }

                // Only check relevant days in this month
                let current_date = if year == start_year && month == start.month() {
                    start.date_naive()
                } else {
                    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
                };

                let month_end = if year == end_year && month == end.month() {
                    end.date_naive()
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1)
                        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                        .pred_opt().unwrap()
                };

                let mut current_date = current_date;
                while current_date <= month_end {
                    let file_path = month_path.join(format!("{:04}{:02}{:02}.bin",
                        year,
                        month,
                        current_date.day()
                    ));

                    if file_path.exists() {
                        file_paths.push(file_path);
                    }

                    current_date = match current_date.succ_opt() {
                        Some(date) => date,
                        None => {
                            eprintln!("Failed to get next day");
                            break
                        },
                    }
                }
            }
        }

        Ok(file_paths)
    }

    pub async fn get_compressed_files_in_range(
        &self,
        subscription: Vec<DataSubscription>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Vec<u8>>, FundForgeError> {
        let mut files_data = Vec::new();

        for subscription in subscription {
            let file_paths = self.get_files_in_range(&subscription.symbol, &subscription.resolution, &subscription.base_data_type, start, end).await?;

            for file_path in file_paths {
                let mut file = match File::open(&file_path) {
                    Ok(file) => file,
                    Err(e) => {
                        return Err(FundForgeError::ServerErrorDebug(format!("Error opening file {:?}: {}", file_path, e)));
                    }
                };
                let mut compressed_data = Vec::new();
                match file.read_to_end(&mut compressed_data) {
                    Ok(_) => {},
                    Err(e) => {
                        return Err(FundForgeError::ServerErrorDebug(format!("Error reading file {:?}: {}", file_path, e)));
                    }
                }

                files_data.push(compressed_data);
            }
        }

        Ok(files_data)
    }

    // Updated get_latest_data_point function
    pub async fn get_latest_data_point(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<BaseDataEnum>, Box<dyn std::error::Error>> {
        let base_path = self.get_base_path(symbol, resolution, data_type, false);
        if !base_path.exists() {
            return Ok(None);
        }

        let mut latest_point: Option<BaseDataEnum> = None;

        // Get years in descending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|s| s.parse::<i32>().ok())
                    .map(|year| (year, e.path()))
            })
            .collect();
        years.sort_unstable_by(|a, b| b.0.cmp(&a.0));  // Sort descending

        'outer: for (_year, year_path) in years {
            let mut months: Vec<_> = fs::read_dir(year_path)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|month| (month, e.path()))
                })
                .collect();
            months.sort_unstable_by(|a, b| b.0.cmp(&a.0));  // Sort descending

            for (_month, month_path) in months {
                let mut days: Vec<_> = fs::read_dir(month_path)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
                    .collect();
                days.sort_by_key(|e| std::cmp::Reverse(e.path()));

                for day in days {
                    let file_path = day.path();
                    if let Ok(mmap) = self.get_or_create_mmap(&file_path).await {
                        match self.decompress_and_deserialize(&mmap[..]).await {
                            Ok(day_data) => {
                                if let Some(latest) = day_data.into_iter()
                                    .max_by_key(|d| d.time_closed_utc()) {
                                    latest_point = Some(latest);
                                    break 'outer;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error processing file {}: {}", file_path.display(), e);
                                continue;
                            }
                        }
                    }
                }
            }
        }

        Ok(latest_point)
    }

    pub async fn get_latest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        let base_path = self.get_base_path(symbol, resolution, data_type, false);
        if !base_path.exists() {
            return Ok(None);
        }

        let mut latest_time: Option<DateTime<Utc>> = None;

        // Get years in descending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|s| s.parse::<i32>().ok())
                    .map(|year| (year, e.path()))
            })
            .collect();
        years.sort_unstable_by(|a, b| b.0.cmp(&a.0));  // Sort descending

        for (_year, year_path) in years {
            // Get months in descending order
            let mut months: Vec<_> = fs::read_dir(year_path)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|month| (month, e.path()))
                })
                .collect();
            months.sort_unstable_by(|a, b| b.0.cmp(&a.0));  // Sort descending

            for (_month, month_path) in months {
                // Get days in descending order
                let mut days: Vec<_> = fs::read_dir(month_path)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "bin"))
                    .collect();
                days.sort_by_key(|e| std::cmp::Reverse(e.path()));

                for day in days {
                    let file_path = day.path();
                    // Read file directly instead of using mmap
                    let mut file = match File::open(&file_path) {
                        Ok(file) => file,
                        Err(e) => {
                            eprintln!("Error opening file {}: {}", file_path.display(), e);
                            continue;
                        }
                    };

                    let mut compressed_data = Vec::new();
                    if let Err(e) = file.read_to_end(&mut compressed_data) {
                        eprintln!("Error reading file {}: {}", file_path.display(), e);
                        continue;
                    }

                    // Decompress
                    let mut decoder = GzDecoder::new(&compressed_data[..]);
                    let mut decompressed = Vec::new();
                    match decoder.read_to_end(&mut decompressed) {
                        Ok(_) => {
                            match BaseDataEnum::from_array_bytes(&decompressed) {
                                Ok(day_data) => {
                                    if let Some(time) = day_data.into_iter()
                                        .map(|d| d.time_closed_utc())
                                        .max() {
                                        match latest_time {
                                            None => latest_time = Some(time),
                                            Some(current_latest) if time > current_latest => {
                                                latest_time = Some(time)
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error deserializing data from {}: {}", file_path.display(), e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error decompressing data from {}: {}", file_path.display(), e);
                            continue;
                        }
                    }
                }
            }
        }

        Ok(latest_time)
    }

    pub async fn get_bulk_data(
        &self,
        subscriptions: &[DataSubscription],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<BTreeMap<i64, TimeSlice>, FundForgeError> {
        let tasks: Vec<_> = subscriptions
            .iter()
            .map(|subscription| {
                let symbol = subscription.symbol.clone();
                let resolution = subscription.resolution.clone();
                let base_data_type = subscription.base_data_type.clone();

                async move {
                    self.get_data_range(&symbol, &resolution, &base_data_type, start, end).await
                }
            })
            .collect();

        let results = future::join_all(tasks).await;

        let mut combined_data: BTreeMap<i64, TimeSlice> = BTreeMap::new();

        for result in results {
            match result {
                Ok(data) => {
                    for item in data {
                        let entry = combined_data
                            .entry(item.time_closed_utc().timestamp_nanos_opt().unwrap())
                            .or_insert_with(TimeSlice::new);

                        entry.add(item);
                    }
                }
                Err(e) => return Err(FundForgeError::ServerErrorDebug(e.to_string())),
            }
        }

        Ok(combined_data)
    }

    // Updated get_data_point_asof function
    pub async fn get_data_point_asof(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
        target_time: DateTime<Utc>,
    ) -> Result<Option<BaseDataEnum>, FundForgeError> {
        // Get the file path for the target date
        let file_path = self.get_file_path(symbol, resolution, data_type, &target_time, false);

        // If the file exists for the target date, check it first
        if file_path.exists() {
            if let Ok(mmap) = self.get_or_create_mmap(&file_path).await {
                if let Ok(day_data) = self.decompress_and_deserialize(&mmap[..]).await {
                    // Find the closest point before or at target_time
                    let result = day_data.into_iter()
                        .filter(|d| d.time_closed_utc() <= target_time)
                        .max_by_key(|d| d.time_closed_utc());

                    if let Some(point) = result {
                        return Ok(Some(point));
                    }
                }
            }
        }

        let mut current_date = target_time.date_naive();
        let mut attempts = 0;
        const MAX_ATTEMPTS: i32 = 30; // Limit how far back we'll look

        while let Some(prev_date) = current_date.pred_opt() {
            attempts += 1;
            if attempts > MAX_ATTEMPTS {
                break;
            }

            current_date = prev_date;
            let file_path = self.get_file_path(
                symbol,
                resolution,
                data_type,
                &DateTime::<Utc>::from_naive_utc_and_offset(
                    current_date.and_hms_opt(0, 0, 0).unwrap(),
                    Utc,
                ),
                false,
            );

            if !file_path.exists() {
                // Check if we've moved to a different month/year directory
                let month_path = file_path.parent().unwrap();
                let year_path = month_path.parent().unwrap();

                // If neither the month nor year directory exists, break to avoid unnecessary checks
                if !month_path.exists() && !year_path.exists() {
                    break;
                }
                continue;
            }

            if let Ok(mmap) = self.get_or_create_mmap(&file_path).await {
                match self.decompress_and_deserialize(&mmap[..]).await {
                    Ok(day_data) => {
                        let result = day_data.into_iter()
                            .filter(|d| d.time_closed_utc() <= target_time)
                            .max_by_key(|d| d.time_closed_utc());

                        if let Some(point) = result {
                            return Ok(Some(point));
                        }
                    }
                    Err(e) => {
                        eprintln!("Error processing file {}: {}", file_path.display(), e);
                        continue;
                    }
                }
            }
        }

        Ok(None)
    }
}

#[derive(Deserialize)]
struct DownloadSymbols {
    symbols: Vec<DownloadConfig>,
}

#[derive(Deserialize, Debug)]
pub struct DownloadConfig {
    pub symbol_name: SymbolName,
    pub base_data_type: BaseDataType,
    pub start_date: NaiveDate,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub resolution: Resolution
}

// Add this helper function
fn deserialize_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::time::Duration;
    use chrono::{TimeZone, Utc};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use ff_standard_lib::standardized_types::base_data::quote::Quote;

    // Helper function to create test data using Quote with proper decimal values
    fn create_test_data(time: DateTime<Utc>) -> BaseDataEnum {
        BaseDataEnum::Quote(Quote {
            symbol: Symbol::new(
                "EUR/USD".to_string(),
                DataVendor::Test,
                MarketType::Forex
            ),
            ask: dec!(1.2345),
            bid: dec!(1.2343),
            ask_volume: dec!(100000.0),
            bid_volume: dec!(150000.0),
            time: time.to_string(),
        })
    }

    // Helper to create test storage
    fn setup_test_storage() -> (HybridStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let options = ServerLaunchOptions {
            data_folder: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = HybridStorage::new(
            Duration::from_secs(3600),
            options,
            5,
            300
        );

        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_save_and_retrieve_single_quote() {
        let (storage, _temp) = setup_test_storage();
        let time = Utc::now();
        let test_data = create_test_data(time);

        // Save data
        storage.save_data(&test_data).await.unwrap();

        // Retrieve data
        let retrieved = storage.get_data_range(
            test_data.symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes,
            time - chrono::Duration::hours(1),
            time + chrono::Duration::hours(1)
        ).await.unwrap();

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0], test_data);
    }

    #[tokio::test]
    async fn test_bulk_save_and_retrieve() {
        let (storage, _temp) = setup_test_storage();
        let base_time = Utc::now();

        // Create multiple quotes with increasing spreads
        let test_data: Vec<BaseDataEnum> = (0..10)
            .map(|i| {
                let time = base_time + chrono::Duration::seconds(i);
                BaseDataEnum::Quote(Quote {
                    symbol: Symbol::new(
                        "EUR/USD".to_string(),
                        DataVendor::Test,
                        MarketType::Forex
                    ),
                    ask: dec!(1.2345) + dec!(0.0001) * Decimal::from(i),
                    bid: dec!(1.2343),
                    ask_volume: dec!(100000.0),
                    bid_volume: dec!(150000.0),
                    time: time.to_string(),
                })
            })
            .collect();

        // Save bulk data
        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        // Retrieve data range
        let retrieved = storage.get_data_range(
            test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes,
            base_time,
            base_time + chrono::Duration::seconds(10)
        ).await.unwrap();

        assert_eq!(retrieved.len(), 10);
        assert_eq!(retrieved, test_data);
    }

    #[tokio::test]
    async fn test_latest_data_point() {
        let (storage, _temp) = setup_test_storage();
        let base_time = Utc::now();

        let test_data: Vec<BaseDataEnum> = (0..5)
            .map(|i| {
                let time = base_time + chrono::Duration::seconds(i);
                BaseDataEnum::Quote(Quote {
                    symbol: Symbol::new(
                        "EUR/USD".to_string(),
                        DataVendor::Test,
                        MarketType::Forex
                    ),
                    ask: dec!(1.2345),
                    bid: dec!(1.2343),
                    ask_volume: dec!(100000.0),
                    bid_volume: dec!(150000.0),
                    time: time.to_string(),
                })
            })
            .collect();

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let latest = storage.get_latest_data_point(
            test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert!(latest.is_some());
        assert_eq!(latest.unwrap(), test_data.last().unwrap().clone());
    }

    #[tokio::test]
    async fn test_earliest_data_point() {
        let (storage, _temp) = setup_test_storage();
        let base_time = Utc::now();

        let test_data: Vec<BaseDataEnum> = (0..5)
            .map(|i| {
                let time = base_time + chrono::Duration::seconds(i);
                BaseDataEnum::Quote(Quote {
                    symbol: Symbol::new(
                        "EUR/USD".to_string(),
                        DataVendor::Test,
                        MarketType::Forex
                    ),
                    ask: dec!(1.2345),
                    bid: dec!(1.2343),
                    ask_volume: dec!(100000.0),
                    bid_volume: dec!(150000.0),
                    time: time.to_string(),
                })
            })
            .collect();

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let earliest = storage.get_earliest_data_time(
            test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert!(earliest.is_some());
        assert_eq!(
            earliest.unwrap(),
            base_time,
            "Earliest time should match the first data point's time"
        );
    }

    #[tokio::test]
    async fn test_no_duplicate_saves() {
        let (storage, _temp) = setup_test_storage();
        let time = Utc::now();

        // Create two identical quotes with the same timestamp
        let quote1 = BaseDataEnum::Quote(Quote {
            symbol: Symbol::new(
                "EUR/USD".to_string(),
                DataVendor::Test,
                MarketType::Forex
            ),
            ask: dec!(1.2345),
            bid: dec!(1.2343),
            ask_volume: dec!(100000.0),
            bid_volume: dec!(150000.0),
            time: time.to_string(),
        });

        let quote2 = BaseDataEnum::Quote(Quote {
            symbol: Symbol::new(
                "EUR/USD".to_string(),
                DataVendor::Test,
                MarketType::Forex
            ),
            ask: dec!(1.2346), // Different price
            bid: dec!(1.2344), // Different price
            ask_volume: dec!(100000.0),
            bid_volume: dec!(150000.0),
            time: time.to_string(), // Same time
        });

        // Save first quote
        storage.save_data(&quote1).await.unwrap();

        // Try to save second quote with same timestamp
        storage.save_data(&quote2).await.unwrap();

        // Retrieve data for the time period
        let retrieved = storage.get_data_range(
            quote1.symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes,
            time - chrono::Duration::seconds(1),
            time + chrono::Duration::seconds(1)
        ).await.unwrap();

        // Should only have one quote - the latest one
        assert_eq!(retrieved.len(), 1, "Should only have one quote for the timestamp");
        assert_eq!(retrieved[0], quote2, "Should have the latest quote for the timestamp");
    }

    #[tokio::test]
    async fn test_bulk_save_deduplication() {
        let (storage, _temp) = setup_test_storage();
        let time = Utc::now();

        // Create a vector of quotes with some duplicate timestamps
        let mut test_data = Vec::new();

        // Add first quote
        test_data.push(BaseDataEnum::Quote(Quote {
            symbol: Symbol::new(
                "EUR/USD".to_string(),
                DataVendor::Test,
                MarketType::Forex
            ),
            ask: dec!(1.2345),
            bid: dec!(1.2343),
            ask_volume: dec!(100000.0),
            bid_volume: dec!(150000.0),
            time: time.to_string(),
        }));

        // Add second quote with same timestamp but different prices
        test_data.push(BaseDataEnum::Quote(Quote {
            symbol: Symbol::new(
                "EUR/USD".to_string(),
                DataVendor::Test,
                MarketType::Forex
            ),
            ask: dec!(1.2346),
            bid: dec!(1.2344),
            ask_volume: dec!(100000.0),
            bid_volume: dec!(150000.0),
            time: time.to_string(),
        }));

        // Add a quote with different timestamp
        test_data.push(BaseDataEnum::Quote(Quote {
            symbol: Symbol::new(
                "EUR/USD".to_string(),
                DataVendor::Test,
                MarketType::Forex
            ),
            ask: dec!(1.2347),
            bid: dec!(1.2345),
            ask_volume: dec!(100000.0),
            bid_volume: dec!(150000.0),
            time: (time + chrono::Duration::seconds(1)).to_string(),
        }));

        // Save all data
        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        // Retrieve data for the time period
        let retrieved = storage.get_data_range(
            test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes,
            time - chrono::Duration::seconds(1),
            time + chrono::Duration::seconds(2)
        ).await.unwrap();

        // Should have 2 quotes - one for each unique timestamp
        assert_eq!(retrieved.len(), 2, "Should have one quote per unique timestamp");

        // First quote should be the latest one for the duplicate timestamp
        assert_eq!(retrieved[0].time_closed_utc(), time);
        assert_eq!(retrieved[0], test_data[1]); // Second quote (latest) for first timestamp

        // Second quote should be the one with the different timestamp
        assert_eq!(retrieved[1].time_closed_utc(), time + chrono::Duration::seconds(1));
        assert_eq!(retrieved[1], test_data[2]);
    }

    #[tokio::test]
    async fn test_earliest_data_across_days() {
        let (storage, _temp) = setup_test_storage();
        println!("Test directory: {:?}", _temp.path());

        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        println!("Base time: {}", base_time);

        // Create data points across different days
        let test_data: Vec<BaseDataEnum> = (0..48) // 2 days worth of hourly data
            .map(|i| {
                let time = base_time + chrono::Duration::hours(i);
                BaseDataEnum::Quote(Quote {
                    symbol: Symbol::new(
                        "EUR/USD".to_string(), // Keep the original format
                        DataVendor::Test,
                        MarketType::Forex
                    ),
                    ask: dec!(1.2345) + dec!(0.0001) * Decimal::from(i),
                    bid: dec!(1.2343) + dec!(0.0001) * Decimal::from(i),
                    ask_volume: dec!(100000.0),
                    bid_volume: dec!(150000.0),
                    time: time.to_string(),
                })
            })
            .collect();

        println!("Created {} test data points", test_data.len());
        println!("First data point time: {}", test_data[0].time_closed_utc());
        println!("Last data point time: {}", test_data[47].time_closed_utc());

        // Save data
        match storage.save_data_bulk(test_data.clone(), false).await {
            Ok(_) => println!("Successfully saved bulk data"),
            Err(e) => println!("Error saving bulk data: {:?}", e),
        }

        // Check the base path
        let symbol = test_data[0].symbol();
        let base_path = storage.get_base_path(
            symbol,
            &Resolution::Instant,
            &BaseDataType::Quotes,
            false
        );
        println!("Looking for data in base path: {:?}", base_path);
        println!("Base path exists: {}", base_path.exists());

        if base_path.exists() {
            // List contents of directory
            println!("Directory contents:");
            for entry in std::fs::read_dir(&base_path).unwrap() {
                println!("  {:?}", entry.unwrap().path());
            }
        }

        // Try to get earliest time with error logging
        let earliest = match storage.get_earliest_data_time(
            test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await {
            Ok(Some(time)) => {
                println!("Found earliest time: {}", time);
                Some(time)
            },
            Ok(None) => {
                println!("No earliest time found");
                None
            },
            Err(e) => {
                println!("Error getting earliest time: {:?}", e);
                None
            }
        };

        assert!(earliest.is_some(), "Expected earliest data point to exist");
        assert_eq!(
            earliest.unwrap(),
            base_time,
            "Earliest time should match the first data point across multiple days"
        );
    }

    // Helper function to verify data was saved correctly
    async fn verify_data_saved(storage: &HybridStorage, symbol: &Symbol, time: DateTime<Utc>) -> bool {
        let file_path = storage.get_file_path(
            symbol,
            &Resolution::Instant,
            &BaseDataType::Quotes,
            &time,
            false
        );
        file_path.exists()
    }

    #[tokio::test]
    async fn test_get_data_point_asof() {
        let (storage, _temp) = setup_test_storage();
        let base_time = Utc::now();

        // Create test data with known timestamps
        let test_data: Vec<BaseDataEnum> = vec![
            // T+0
            BaseDataEnum::Quote(Quote {
                symbol: Symbol::new(
                    "EUR-USD".to_string(),
                    DataVendor::Test,
                    MarketType::Forex
                ),
                ask: dec!(1.2345),
                bid: dec!(1.2343),
                ask_volume: dec!(100000.0),
                bid_volume: dec!(150000.0),
                time: base_time.to_string(),
            }),
            // T+2
            BaseDataEnum::Quote(Quote {
                symbol: Symbol::new(
                    "EUR-USD".to_string(),
                    DataVendor::Test,
                    MarketType::Forex
                ),
                ask: dec!(1.2346),
                bid: dec!(1.2344),
                ask_volume: dec!(100000.0),
                bid_volume: dec!(150000.0),
                time: (base_time + chrono::Duration::seconds(2)).to_string(),
            }),
            // T+5
            BaseDataEnum::Quote(Quote {
                symbol: Symbol::new(
                    "EUR-USD".to_string(),
                    DataVendor::Test,
                    MarketType::Forex
                ),
                ask: dec!(1.2347),
                bid: dec!(1.2345),
                ask_volume: dec!(100000.0),
                bid_volume: dec!(150000.0),
                time: (base_time + chrono::Duration::seconds(5)).to_string(),
            }),
        ];

        // Save the test data
        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        // Test cases
        let test_cases = vec![
            // Exact match at T+0
            (base_time, Some(&test_data[0])),
            // Between T+0 and T+2 should return T+0
            (base_time + chrono::Duration::seconds(1), Some(&test_data[0])),
            // Exact match at T+2
            (base_time + chrono::Duration::seconds(2), Some(&test_data[1])),
            // Between T+2 and T+5 should return T+2
            (base_time + chrono::Duration::seconds(3), Some(&test_data[1])),
            // Exact match at T+5
            (base_time + chrono::Duration::seconds(5), Some(&test_data[2])),
            // After T+5 should return T+5
            (base_time + chrono::Duration::seconds(6), Some(&test_data[2])),
            // Before all data should return None
            (base_time - chrono::Duration::seconds(1), None),
        ];

        for (query_time, expected_result) in test_cases {
            let result = storage.get_data_point_asof(
                &test_data[0].symbol().clone(),
                &Resolution::Instant,
                &BaseDataType::Quotes,
                query_time
            ).await.unwrap();

            match (result, expected_result) {
                (Some(actual), Some(expected)) => assert_eq!(actual, *expected),
                (None, None) => (),
                _ => panic!("Result mismatch for query time {:?}", query_time),
            }
        }
    }

    #[tokio::test]
    async fn test_get_data_point_asof_across_days() {
        let (storage, _temp) = setup_test_storage();
        let day1 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let day2 = Utc.with_ymd_and_hms(2024, 1, 2, 12, 0, 0).unwrap();

        // Create test data across two days
        let test_data = vec![
            BaseDataEnum::Quote(Quote {
                symbol: Symbol::new("EUR-USD".to_string(), DataVendor::Test, MarketType::Forex),
                ask: dec!(1.2345),
                bid: dec!(1.2343),
                ask_volume: dec!(100000.0),
                bid_volume: dec!(150000.0),
                time: day1.to_string(),
            }),
            BaseDataEnum::Quote(Quote {
                symbol: Symbol::new("EUR-USD".to_string(), DataVendor::Test, MarketType::Forex),
                ask: dec!(1.2346),
                bid: dec!(1.2344),
                ask_volume: dec!(100000.0),
                bid_volume: dec!(150000.0),
                time: day2.to_string(),
            }),
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        // Query for a time between the two days
        let query_time = Utc.with_ymd_and_hms(2024, 1, 2, 1, 0, 0).unwrap();
        let result = storage.get_data_point_asof(
            &test_data[0].symbol().clone(),
            &Resolution::Instant,
            &BaseDataType::Quotes,
            query_time
        ).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), test_data[0]);
    }

    fn create_test_quote(time: DateTime<Utc>, ask: Decimal, bid: Decimal) -> BaseDataEnum {
        BaseDataEnum::Quote(Quote {
            symbol: Symbol::new(
                "EUR-USD".to_string(),
                DataVendor::Test,
                MarketType::Forex
            ),
            ask,
            bid,
            ask_volume: dec!(100000.0),
            bid_volume: dec!(150000.0),
            time: time.to_string(),
        })
    }

    #[tokio::test]
    async fn test_earliest_time_basic() {
        let (storage, _temp) = setup_test_storage();

        // Create test data with known timestamps
        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let test_data = vec![
            create_test_quote(base_time + chrono::Duration::hours(2), dec!(1.2345), dec!(1.2343)),
            create_test_quote(base_time + chrono::Duration::hours(1), dec!(1.2346), dec!(1.2344)),
            create_test_quote(base_time, dec!(1.2347), dec!(1.2345)),
        ];

        // Save data
        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        // Get earliest time
        let earliest = storage.get_earliest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(earliest, Some(base_time), "Earliest time should be base_time");
    }

    #[tokio::test]
    async fn test_latest_time_basic() {
        let (storage, _temp) = setup_test_storage();

        let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let latest_time = base_time + chrono::Duration::hours(2);

        let test_data = vec![
            create_test_quote(base_time, dec!(1.2345), dec!(1.2343)),
            create_test_quote(base_time + chrono::Duration::hours(1), dec!(1.2346), dec!(1.2344)),
            create_test_quote(latest_time, dec!(1.2347), dec!(1.2345)),
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let result = storage.get_latest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(result, Some(latest_time), "Latest time should be latest_time");
    }

    #[tokio::test]
    async fn test_earliest_time_across_days() {
        let (storage, _temp) = setup_test_storage();

        let day1 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let day2 = Utc.with_ymd_and_hms(2024, 1, 2, 12, 0, 0).unwrap();
        let day3 = Utc.with_ymd_and_hms(2024, 1, 3, 12, 0, 0).unwrap();

        let test_data = vec![
            create_test_quote(day2, dec!(1.2345), dec!(1.2343)),
            create_test_quote(day3, dec!(1.2346), dec!(1.2344)),
            create_test_quote(day1, dec!(1.2347), dec!(1.2345)), // Earliest
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let earliest = storage.get_earliest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(earliest, Some(day1), "Earliest time should be day1");
    }

    #[tokio::test]
    async fn test_latest_time_across_days() {
        let (storage, _temp) = setup_test_storage();

        let day1 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let day2 = Utc.with_ymd_and_hms(2024, 1, 2, 12, 0, 0).unwrap();
        let day3 = Utc.with_ymd_and_hms(2024, 1, 3, 12, 0, 0).unwrap();

        let test_data = vec![
            create_test_quote(day1, dec!(1.2345), dec!(1.2343)),
            create_test_quote(day2, dec!(1.2346), dec!(1.2344)),
            create_test_quote(day3, dec!(1.2347), dec!(1.2345)), // Latest
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let latest = storage.get_latest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(latest, Some(day3), "Latest time should be day3");
    }

    #[tokio::test]
    async fn test_earliest_time_across_months() {
        let (storage, _temp) = setup_test_storage();

        let month1 = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let month2 = Utc.with_ymd_and_hms(2024, 2, 15, 12, 0, 0).unwrap();
        let month3 = Utc.with_ymd_and_hms(2024, 3, 15, 12, 0, 0).unwrap();

        let test_data = vec![
            create_test_quote(month2, dec!(1.2345), dec!(1.2343)),
            create_test_quote(month3, dec!(1.2346), dec!(1.2344)),
            create_test_quote(month1, dec!(1.2347), dec!(1.2345)), // Earliest
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let earliest = storage.get_earliest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(earliest, Some(month1), "Earliest time should be month1");
    }

    #[tokio::test]
    async fn test_latest_time_across_months() {
        let (storage, _temp) = setup_test_storage();

        let month1 = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let month2 = Utc.with_ymd_and_hms(2024, 2, 15, 12, 0, 0).unwrap();
        let month3 = Utc.with_ymd_and_hms(2024, 3, 15, 12, 0, 0).unwrap();

        let test_data = vec![
            create_test_quote(month1, dec!(1.2345), dec!(1.2343)),
            create_test_quote(month2, dec!(1.2346), dec!(1.2344)),
            create_test_quote(month3, dec!(1.2347), dec!(1.2345)), // Latest
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let latest = storage.get_latest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(latest, Some(month3), "Latest time should be month3");
    }

    #[tokio::test]
    async fn test_earliest_time_empty_storage() {
        let (storage, _temp) = setup_test_storage();

        let result = storage.get_earliest_data_time(
            &Symbol::new("EUR-USD".to_string(), DataVendor::Test, MarketType::Forex),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(result, None, "Earliest time should be None for empty storage");
    }

    #[tokio::test]
    async fn test_latest_time_empty_storage() {
        let (storage, _temp) = setup_test_storage();

        let result = storage.get_latest_data_time(
            &Symbol::new("EUR-USD".to_string(), DataVendor::Test, MarketType::Forex),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(result, None, "Latest time should be None for empty storage");
    }

    #[tokio::test]
    async fn test_earliest_time_with_gaps() {
        let (storage, _temp) = setup_test_storage();

        // Create data with time gaps
        let early_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let mid_time = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let late_time = Utc.with_ymd_and_hms(2024, 1, 30, 0, 0, 0).unwrap();

        let test_data = vec![
            create_test_quote(mid_time, dec!(1.2345), dec!(1.2343)),
            create_test_quote(late_time, dec!(1.2346), dec!(1.2344)),
            create_test_quote(early_time, dec!(1.2347), dec!(1.2345)), // Earliest
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let earliest = storage.get_earliest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(earliest, Some(early_time), "Should find earliest time even with gaps");
    }

    #[tokio::test]
    async fn test_latest_time_with_gaps() {
        let (storage, _temp) = setup_test_storage();

        // Create data with time gaps
        let early_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let mid_time = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let late_time = Utc.with_ymd_and_hms(2024, 1, 30, 0, 0, 0).unwrap();

        let test_data = vec![
            create_test_quote(early_time, dec!(1.2345), dec!(1.2343)),
            create_test_quote(mid_time, dec!(1.2346), dec!(1.2344)),
            create_test_quote(late_time, dec!(1.2347), dec!(1.2345)), // Latest
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let latest = storage.get_latest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(latest, Some(late_time), "Should find latest time even with gaps");
    }

    #[tokio::test]
    async fn test_earliest_and_latest_time_single_point() {
        let (storage, _temp) = setup_test_storage();

        let time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let test_data = vec![
            create_test_quote(time, dec!(1.2345), dec!(1.2343)),
        ];

        storage.save_data_bulk(test_data.clone(), false).await.unwrap();

        let earliest = storage.get_earliest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        let latest = storage.get_latest_data_time(
            &test_data[0].symbol(),
            &Resolution::Instant,
            &BaseDataType::Quotes
        ).await.unwrap();

        assert_eq!(earliest, Some(time), "Earliest time should match single point");
        assert_eq!(latest, Some(time), "Latest time should match single point");
    }
}