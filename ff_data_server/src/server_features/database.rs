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
use futures::future;
use indicatif::{MultiProgress, ProgressBar};
use memmap2::{Mmap};
use serde::{Deserialize, Deserializer};
use tokio::sync::{OnceCell, Semaphore};
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::interval;
use ff_standard_lib::messages::data_server_messaging::{FundForgeError};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::datavendor_enum::DataVendor;
use ff_standard_lib::standardized_types::enums::MarketType;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use crate::oanda_api::api_client::{OANDA_CLIENT, OANDA_IS_CONNECTED};
use crate::rithmic_api::api_client::{get_rithmic_market_data_system, RITHMIC_CLIENTS, RITHMIC_DATA_IS_CONNECTED};
use ff_standard_lib::product_maps::rithmic::maps::get_exchange_by_symbol_name;
use crate::server_features::server_side_datavendor::VendorApiResponse;
use crate::{get_data_folder, subscribe_server_shutdown, ServerLaunchOptions};

pub static DATA_STORAGE: OnceCell<Arc<HybridStorage>> = OnceCell::const_new();

#[allow(unused)]
pub struct HybridStorage {
    base_path: PathBuf,
    mmap_cache: Arc<DashMap<String, Arc<Mmap>>>,
    cache_last_accessed: Arc<DashMap<String, DateTime<Utc>>>,
    cache_is_updated: Arc<DashMap<String, bool>>,
    clear_cache_duration: Duration,
    download_tasks: Arc<DashMap<(SymbolName, BaseDataType, Resolution), JoinHandle<()>>>,
    options: ServerLaunchOptions,
    multi_bar: MultiProgress,
    download_semaphore: Arc<Semaphore>,
    update_seconds: u64,
}

impl HybridStorage {
    pub fn new(clear_cache_duration: Duration, options: ServerLaunchOptions, max_concurrent_downloads: usize, update_seconds: u64) -> Self {
        let storage = Self {
            base_path: options.data_folder.clone().join("historical"),
            mmap_cache: Arc::new(DashMap::new()),
            cache_last_accessed: Arc::new(DashMap::new()),
            cache_is_updated: Arc::new(DashMap::new()),
            clear_cache_duration,
            download_tasks:Default::default(),
            options,
            multi_bar: MultiProgress::new(),
            download_semaphore: Arc::new(Semaphore::new(max_concurrent_downloads)),
            update_seconds
        };

        // Start the background task for cache management
        storage.start_cache_management();

        storage
    }

    pub fn run_update_schedule(self: Arc<Self>) {
        //println!("Initializing update schedule with {} second interval", self.update_seconds);
        let mut shutdown_receiver = subscribe_server_shutdown();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(self.update_seconds));
            interval.tick().await;

            // Initial update on startup - first forward then backward
            match HybridStorage::update_data(self.clone(), false).await {
                Ok(_) => { }
                Err(e) => eprintln!("Initial forward update failed: {}", e),
            }
            // Run backward update after forward completes
            match HybridStorage::update_data(self.clone(), true).await {
                Ok(_) => {},
                Err(e) => eprintln!("Initial backward update failed: {}", e),
            }

            loop {
                tokio::select! {
                    _ = shutdown_receiver.recv() => {
                        println!("Received shutdown signal, stopping update schedule");
                        for task in self.download_tasks.iter() {
                            task.value().abort();
                        }
                        break;
                    }
                    _ = interval.tick() => {
                        if !self.download_tasks.is_empty() {
                            tokio::time::sleep(Duration::from_secs(60)).await;
                            interval = tokio::time::interval(Duration::from_secs(60));
                            continue;
                        } else {
                            interval = tokio::time::interval(Duration::from_secs(self.update_seconds));
                        }

                        // Run forward update
                        match HybridStorage::update_data(self.clone(), false).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Forward update failed: {}", e),
                        }

                        // Wait for all forward tasks to complete
                        while !self.download_tasks.is_empty() {
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }

                        // Run backward update after forward completes
                        match HybridStorage::update_data(self.clone(), true).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Backward update failed: {}", e),
                        }
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

        let permit = match self.download_semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                // Remove from tasks if we couldn't get a permit
                self.download_tasks.remove(&key);
                return;
            }
        };

        let symbol_pb = self.multi_bar.add(ProgressBar::new(1));
        symbol_pb.set_prefix(format!("{}", symbol.name));

        match client.update_historical_data(symbol.clone(), base_data_type, resolution, start_time, Utc::now() + Duration::from_secs(15), false, symbol_pb).await {
            Ok(_) => {},
            Err(_) => {}
        }

        // Remove from active tasks
        self.download_tasks.remove(&key);
        drop(permit);
    }

    async fn update_symbol(
        download_tasks: Arc<DashMap<(SymbolName, BaseDataType, Resolution), JoinHandle<()>>>,
        download_semaphore: Arc<Semaphore>,
        symbol: Symbol,
        resolution: Resolution,
        base_data_type: BaseDataType,
        symbol_pb: ProgressBar,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        from_back: bool,
    ) -> Option<JoinHandle<()>> {
        if download_tasks.contains_key(&(symbol.name.clone(), base_data_type, resolution)) {
            return None;
        }
        let client: Arc<dyn VendorApiResponse> = match symbol.data_vendor {
            DataVendor::Rithmic if RITHMIC_DATA_IS_CONNECTED.load(Ordering::SeqCst) => {
                match get_rithmic_market_data_system().and_then(|sys| RITHMIC_CLIENTS.get(&sys)) {
                    Some(client) => client.clone(),
                    None => return None,
                }
            }
            DataVendor::Oanda if OANDA_IS_CONNECTED.load(Ordering::SeqCst)=> {
                match OANDA_CLIENT.get() {
                    Some(client) => client.clone(),
                    None => return None,
                }
            }
            _ => return None,
        };

        let key = (symbol.name.clone(), base_data_type.clone(), resolution.clone());
        if download_tasks.contains_key(&key) {
            return None;
        }

        let semaphore = download_semaphore.clone();
        let download_tasks = download_tasks.clone();

        Some(task::spawn(async move {
            let permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    // Remove from tasks if we couldn't get a permit
                    download_tasks.remove(&key);
                    return;
                }
            };

            symbol_pb.set_prefix(format!("{}", symbol.name));

            match client.update_historical_data(symbol.clone(), base_data_type, resolution, from, to, from_back, symbol_pb).await {
                Ok(_) => {},
                Err(_) => {}
            }

            // Remove from active tasks
            download_tasks.remove(&key);
            drop(permit);
        }))
    }

    async fn update_data(self: Arc<Self>, from_back: bool) -> Result<(), FundForgeError> {
        let options = self.options.clone();
        let multi_bar = self.multi_bar.clone();
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
                                Ok(None) => Some(Utc::now()),  // If we have no data
                                _ => None,
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

                        // Only skip if we're moving backwards and we've already reached our target
                        if from_back && start_time >= end_time - Duration::from_secs(60*60*72) {
                            continue;
                        }

                        let semaphore = semaphore.clone();
                        let download_tasks = self.download_tasks.clone();
                        // Directly spawn the update_symbol task
                        // Create and configure symbol progress bar with an initial length
                        let symbol_pb = multi_bar.add(ProgressBar::new(1));
                        if let Some(spawn_handle) = HybridStorage::update_symbol(
                            download_tasks.clone(),
                            semaphore,
                            symbol.clone(),
                            symbol_config.resolution,
                            symbol_config.base_data_type.clone(),
                            symbol_pb,
                            start_time,
                            end_time,
                            from_back
                        ).await {
                            download_tasks.insert((symbol_config.symbol_name, symbol_config.base_data_type, symbol_config.resolution.clone()), spawn_handle);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn start_cache_management(&self) {
        let mmap_cache = Arc::clone(&self.mmap_cache);
        let cache_last_accessed = Arc::clone(&self.cache_last_accessed);
        let cache_is_updated = Arc::clone(&self.cache_is_updated);
        let clear_cache_duration = self.clear_cache_duration;

        task::spawn(async move {
            let mut interval = interval(clear_cache_duration);

            loop {
                interval.tick().await;

                let now = Utc::now();

                mmap_cache.retain(|path, mmap| {
                    if let Some(last_access) = cache_last_accessed.get(path) {
                        if now.signed_duration_since(*last_access) > chrono::Duration::from_std(clear_cache_duration).unwrap() {
                            if let Some(updated) = cache_is_updated.get(path) {
                                if *updated.value() {
                                    // Save the updated mmap to disk
                                    if let Err(e) = Self::save_mmap_to_disk(path, mmap) {
                                        eprintln!("Failed to save mmap to disk: {}", e);
                                    }
                                }
                            }
                            false // Remove from cache
                        } else {
                            true // Keep in cache
                        }
                    } else {
                        false // Remove if no last access time (shouldn't happen)
                    }
                });

                // Clean up the auxiliary hashmaps
                cache_last_accessed.retain(|k, _| mmap_cache.contains_key(k));
                cache_is_updated.retain(|k, _| mmap_cache.contains_key(k));
            }
        });
    }

    fn save_mmap_to_disk(path: &str, mmap: &Arc<Mmap>) -> io::Result<()> {
        let mut file = OpenOptions::new().write(true).open(path)?;
        file.write_all(mmap.as_ref())?;
        file.sync_all()?;
        Ok(())
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
        self.save_data_to_file(&file_path, &[data.clone()]).await
    }

    pub async fn save_data_bulk(&self, data: Vec<BaseDataEnum>) -> io::Result<()> {
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
                d.time_closed_utc().date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap()  //todo[Remove Unwrap]
            );
            grouped_data.entry(key).or_insert_with(Vec::new).push(d);
        }

        for ((symbol, resolution, data_type, date), group) in grouped_data {
            let file_path = self.get_file_path(&symbol, &resolution, &data_type, &date, true);
            self.save_data_to_file(&file_path, &group).await?;
        }

        Ok(())
    }

    async fn save_data_to_file(&self, file_path: &Path, new_data: &[BaseDataEnum]) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(file_path)?;

        let mut existing_data = Vec::new();
        file.read_to_end(&mut existing_data)?;

        let mut data_map: BTreeMap<DateTime<Utc>, BaseDataEnum> = if !existing_data.is_empty() {
            BaseDataEnum::from_array_bytes(&existing_data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                .into_iter()
                .map(|d| (d.time_closed_utc(), d))
                .collect()
        } else {
            BTreeMap::new()
        };

        for data_point in new_data {
            data_map.insert(data_point.time_closed_utc(), data_point.clone());
        }

        let all_data: Vec<BaseDataEnum> = data_map.into_values().collect();
        let bytes = BaseDataEnum::vec_to_bytes(all_data);

        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(&bytes)?;

        let mmap = unsafe { Mmap::map(&file)? };
        self.mmap_cache.insert(file_path.to_string_lossy().to_string(), Arc::new(mmap));

        self.cache_is_updated.insert(file_path.to_string_lossy().to_string(), true);

        Ok(())
    }

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

                // Only iterate through relevant days in this month
                let current_date = if year == start_year && month == start.month() {
                    start.date_naive()
                } else {
                    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
                };
                let month_end = if year == end_year && month == end.month() {
                    end.date_naive()
                } else {
                    // Last day of month
                    NaiveDate::from_ymd_opt(year, month + 1, 1)
                        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                        .pred_opt().unwrap()
                };

                let mut current_date = current_date;
                while current_date <= month_end {
                    let file_path = month_path.join(format!("{:04}{:02}{:02}.bin", year, month, current_date.day()));
                    if let Ok(mmap) = self.get_or_create_mmap(&file_path) {
                        let day_data = BaseDataEnum::from_array_bytes(&mmap[..].to_vec()).unwrap();
                        all_data.extend(day_data.into_iter().filter(|d| d.time_closed_utc() >= start && d.time_closed_utc() <= end));
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

        Ok(all_data)
    }

    /// This function will only check back 10,000 days, it will therefore not work beyond 27.5 years into the past,
    pub async fn get_latest_data_point(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<BaseDataEnum>, Box<dyn std::error::Error>> {
        let base_path = self.get_base_path(symbol, resolution, data_type, false);

        // Get years in descending order
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        years.sort_by(|a, b| b.path().cmp(&a.path()));

        for year_dir in years {
            // Get months in descending order
            let mut months: Vec<_> = fs::read_dir(year_dir.path())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            months.sort_by(|a, b| b.path().cmp(&a.path()));

            for month_dir in months {
                // Get days in descending order
                let mut days: Vec<_> = fs::read_dir(month_dir.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("bin"))
                    .collect();
                days.sort_by(|a, b| b.path().cmp(&a.path()));

                // Check first (latest) file in this month
                if let Some(latest_file) = days.first() {
                    if let Ok(mmap) = self.get_or_create_mmap(&latest_file.path()) {
                        if let Ok(day_data) = BaseDataEnum::from_array_bytes(&mmap.to_vec()) {
                            if let Some(latest) = day_data.into_iter().max_by_key(|d| d.time_closed_utc()) {
                                return Ok(Some(latest));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    pub async fn get_latest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        let base_path = self.get_base_path(symbol, resolution, data_type, false);

        // Get latest year
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        years.sort_by_key(|e| e.path());

        if let Some(year_dir) = years.last() {
            // Get latest month in latest year
            let mut months: Vec<_> = fs::read_dir(year_dir.path())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            months.sort_by_key(|e| e.path());

            if let Some(month_dir) = months.last() {
                // Get latest day file in latest month
                let mut days: Vec<_> = fs::read_dir(month_dir.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("bin"))
                    .collect();
                days.sort_by_key(|e| e.path());

                if let Some(latest_file) = days.last() {
                    if let Ok(mmap) = self.get_or_create_mmap(&latest_file.path()) {
                        if let Ok(day_data) = BaseDataEnum::from_array_bytes(&mmap.to_vec()) {
                            return Ok(day_data.into_iter().map(|d| d.time_closed_utc()).max());
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    pub async fn get_earliest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &Resolution,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        // Start from Jan 1st of the earliest year in base_path
        let base_path = self.get_base_path(symbol, resolution, data_type, false);

        // Get earliest year directory
        let mut entries = tokio::fs::read_dir(&base_path).await?;
        let mut years = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false) {
                years.push(entry);
            }
        }

        if years.is_empty() {
            return Ok(None);
        }

        // Sort to get earliest year
        let years = tokio::task::spawn_blocking(move || {
            let mut years = years;
            years.sort_by_key(|e| e.path());
            years
        }).await?;

        // Start with January 1st of the earliest year
        if let Some(year_dir) = years.first() {
            let year: i32 = year_dir.file_name()
                .to_string_lossy()
                .parse()
                .unwrap_or(2000);

            let mut current_date = DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(year, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            );

            // Try each day until we find data or reach end of year
            while current_date.year() == year {
                let file_path = self.get_file_path(symbol, resolution, data_type, &current_date, false);

                if file_path.exists() {
                    if let Ok(mmap) = self.get_or_create_mmap(&file_path) {
                        if let Ok(day_data) = BaseDataEnum::from_array_bytes(&mmap[..].to_vec()) {
                            if let Some(earliest) = day_data.into_iter()
                                .map(|d| d.time_closed_utc())
                                .min() {
                                return Ok(Some(earliest));
                            }
                        }
                    }
                }

                current_date = DateTime::<Utc>::from_naive_utc_and_offset(
                    current_date.date_naive()
                        .succ_opt() // This properly handles month/year boundaries
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap(),
                    Utc,
                );
            }
        }

        Ok(None)
    }


    fn get_or_create_mmap(&self, file_path: &Path) -> io::Result<Arc<Mmap>> {
        let path_str = file_path.to_string_lossy().to_string();

        if let Some(mmap) = self.mmap_cache.get(&path_str) {
            self.cache_last_accessed.insert(path_str.clone(), Utc::now());
            Ok(Arc::clone(mmap.value()))
        } else {
            let file = File::open(file_path)?;
            let mmap = Arc::new(unsafe { Mmap::map(&file)? });
            self.mmap_cache.insert(path_str.clone(), Arc::clone(&mmap));

            self.cache_last_accessed.insert(path_str.clone(), Utc::now());

            self.cache_is_updated.insert(path_str, false);

            Ok(mmap)
        }
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
