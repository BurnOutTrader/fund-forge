use std::collections::{BTreeMap, HashMap};
use std::fs;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use std::path::{Path, PathBuf};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::sync::{Arc};
use std::time::Duration;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use dashmap::DashMap;
use futures::future;
use futures_util::future::join_all;
use memmap2::{Mmap};
use serde_derive::Deserialize;
use tokio::sync::{OnceCell};
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
use crate::oanda_api::api_client::OANDA_CLIENT;
use crate::rithmic_api::api_client::{get_rithmic_market_data_system, RITHMIC_CLIENTS};
use crate::rithmic_api::products::get_exchange_by_symbol_name;
use crate::server_features::server_side_datavendor::VendorApiResponse;
use crate::ServerLaunchOptions;

#[derive(Eq, PartialEq, Clone, Hash, Debug)]
pub struct UpdateTask {
    pub symbol: Symbol,
    pub resolution: Resolution,
    pub base_data_type: BaseDataType
}

pub static DATA_STORAGE: OnceCell<Arc<HybridStorage>> = OnceCell::const_new();

#[allow(unused)]
pub struct HybridStorage {
    base_path: PathBuf,
    mmap_cache: Arc<DashMap<String, Arc<Mmap>>>,
    cache_last_accessed: Arc<DashMap<String, DateTime<Utc>>>,
    cache_is_updated: Arc<DashMap<String, bool>>,
    clear_cache_duration: Duration,
    download_tasks: Arc<DashMap<UpdateTask, JoinHandle<()>>>,
    options: ServerLaunchOptions
}

impl HybridStorage {
    pub fn new(clear_cache_duration: Duration, options: ServerLaunchOptions) -> Self {
        let storage = Self {
            base_path: options.data_folder.clone().join("historical"),
            mmap_cache: Arc::new(DashMap::new()),
            cache_last_accessed: Arc::new(DashMap::new()),
            cache_is_updated: Arc::new(DashMap::new()),
            clear_cache_duration,
            download_tasks: Arc::new(DashMap::new()),
            options
        };

        // Start the background task for cache management
        storage.start_cache_management();

        storage
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
        let base_path = self.get_base_path(symbol, resolution, data_type, false);

        // Get earliest year
        let mut years: Vec<_> = fs::read_dir(&base_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        years.sort_by_key(|e| e.path());

        if let Some(year_dir) = years.first() {
            // Get earliest month in earliest year
            let mut months: Vec<_> = fs::read_dir(year_dir.path())?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            months.sort_by_key(|e| e.path());

            if let Some(month_dir) = months.first() {
                // Get earliest day file in earliest month
                let mut days: Vec<_> = fs::read_dir(month_dir.path())?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("bin"))
                    .collect();
                days.sort_by_key(|e| e.path());

                if let Some(earliest_file) = days.first() {
                    if let Ok(mmap) = self.get_or_create_mmap(&earliest_file.path()) {
                        if let Ok(day_data) = BaseDataEnum::from_array_bytes(&mmap.to_vec()) {
                            return Ok(day_data.into_iter().map(|d| d.time_closed_utc()).min());
                        }
                    }
                }
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

    pub fn update_history(&self) {
        let options = self.options.clone();
        //let download_tasks = self.download_tasks.clone();
        task::spawn(async move {
            let mut tasks = vec![];
            if options.disable_oanda_server == 0 {
                //update oanda history
                let oanda_path = options.data_folder.clone()
                    .join("credentials")
                    .join("oanda_credentials")
                    .join("download_list.toml");

                if oanda_path.exists() {
                    let content = match std::fs::read_to_string(&oanda_path) {
                        Ok(content) => content,
                        Err(e) => {
                            eprintln!("Failed to read download list file: {}", e);
                            return;
                        }
                    };

                    let symbol_configs = match toml::from_str::<DownloadSymbols>(&content) {
                        Ok(symbol_object) => symbol_object,
                        Err(e) => {
                            eprintln!("Failed to parse download list: {}", e);
                            return;
                        }
                    };
                    //todo, make this into a function, so we can call it on indiviudal symbols
                    if let Some(client) = OANDA_CLIENT.get() {
                        let symbols = symbol_configs.symbols;
                        println!("Oanda Update: Found {} symbols to update", symbols.len());
                        for symbol_config in symbols {
                            if let Some(instrument) = client.instruments.get(&symbol_config.symbol_name) {
                                let symbol = Symbol::new(symbol_config.symbol_name.clone(), DataVendor::Oanda, instrument.market_type);
                                tasks.push(task::spawn(async move {
                                    let resolution = match symbol_config.base_data_type {
                                        BaseDataType::Ticks => Resolution::Ticks(1),
                                        BaseDataType::QuoteBars => Resolution::Seconds(5),
                                        _ => return,
                                    };
                                    match client.update_historical_data_for(symbol, symbol_config.base_data_type, resolution).await {
                                        Ok(_) => println!("Oanda Update: Successfully updated data for: {}", symbol_config.symbol_name),
                                        Err(e) => eprintln!("Oanda Update: Failed to update data for: {} - {}", symbol_config.symbol_name, e),
                                    }
                                }));
                            }
                        }
                    }
                }
            }
            if options.disable_bitget_server == 0 {

            }
            if options.disable_rithmic_server == 0 {
                let rithmic_path = options.data_folder.clone()
                    .join("credentials")
                    .join("rithmic_credentials")
                    .join("download_list.toml");

                if rithmic_path.exists() {
                    let content = match std::fs::read_to_string(&rithmic_path) {
                        Ok(content) => content,
                        Err(e) => {
                            eprintln!("Failed to read download list file: {}", e);
                            return;
                        }
                    };

                    let symbol_configs = match toml::from_str::<DownloadSymbols>(&content) {
                        Ok(symbol_object) => symbol_object,
                        Err(e) => {
                            eprintln!("Failed to parse download list: {}", e);
                            return;
                        }
                    };

                    //todo, make this into a function, so we can call it on indiviudal symbols
                    if let Some(client) = RITHMIC_CLIENTS.get(&get_rithmic_market_data_system()) {
                        let symbols = symbol_configs.symbols;
                        println!("Rithmic Update: Found {} symbols to update", symbols.len());
                        for symbol_config in symbols {
                            let exchange = match get_exchange_by_symbol_name(&symbol_config.symbol_name) {
                                Some(exchange) => exchange,
                                None => {
                                    eprintln!("DataBase Update: Failed to get exchange for symbol: {}", symbol_config.symbol_name);
                                    return
                                },
                            };
                            let symbol = Symbol::new(symbol_config.symbol_name.clone(), DataVendor::Rithmic, MarketType::Futures(exchange));
                            let client = client.clone();
                            tasks.push(task::spawn(async move {
                                let resolution = match symbol_config.base_data_type {
                                    BaseDataType::Ticks => Resolution::Ticks(1),
                                    BaseDataType::Candles => Resolution::Seconds(1),
                                    _ => return,
                                };
                                println!("Updating Rithmic Data for: {}", symbol_config.symbol_name);
                                match client.update_historical_data_for(symbol, symbol_config.base_data_type, resolution).await {
                                    Ok(_) => println!("Rithmic Update: Successfully updated data for: {}", symbol_config.symbol_name),
                                    Err(e) => eprintln!("Rithmic Update: Failed to update data for: {} - {}", symbol_config.symbol_name, e),
                                }
                           }));
                        }
                    }
                }
            }
            join_all(tasks).await;
        });
    }
}

#[derive(Deserialize)]
struct DownloadSymbols {
    symbols: Vec<DownloadConfig>,
}

#[derive(Deserialize)]
struct DownloadConfig {
    symbol_name: SymbolName,
    base_data_type: BaseDataType
}