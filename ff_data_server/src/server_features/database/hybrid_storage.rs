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
    pub static ref MULTIBAR: MultiProgress = MultiProgress::new();
);

#[allow(unused)]
pub struct HybridStorage {
    base_path: PathBuf,
    mmap_cache: Arc<DashMap<String, Arc<Mmap>>>,
    cache_last_accessed: Arc<DashMap<String, DateTime<Utc>>>,
    clear_cache_duration: Duration,
    file_locks: Arc<DashMap<String, Semaphore>>,
    pub(crate) download_tasks: Arc<DashMap<(SymbolName, BaseDataType, Resolution), JoinHandle<()>>>,
    pub(crate) options: ServerLaunchOptions,
    pub(crate) download_semaphore: Arc<Semaphore>,
    pub(crate) update_seconds: u64,
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

    pub(crate) fn get_base_path(&self, symbol: &Symbol, resolution: &Resolution, data_type: &BaseDataType, is_saving: bool) -> PathBuf {
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

    pub(crate) fn get_file_path(&self, symbol: &Symbol, resolution: &Resolution, data_type: &BaseDataType, date: &DateTime<Utc>, is_saving: bool) -> PathBuf {
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

    pub(crate) async fn get_or_create_mmap(&self, file_path: &Path) -> io::Result<Arc<Mmap>> {
        let path_str = file_path.to_string_lossy().to_string();

        if let Some(mmap) = self.mmap_cache.get(&path_str) {
            self.cache_last_accessed.insert(path_str.clone(), Utc::now());
            return Ok(Arc::clone(mmap.value()));
        }

        // Get oldest file outside of any locks
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

        let mut file = File::open(file_path)?;

        // Read compressed data
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
        } // temp_file is dropped here

        // Now open the temp file for mmap
        let temp_file = File::open(&temp_path)?;
        let mmap = Arc::new(unsafe { Mmap::map(&temp_file)? });
        self.mmap_cache.insert(path_str.clone(), Arc::clone(&mmap));
        self.cache_last_accessed.insert(path_str.clone(), Utc::now());

        // Keep the temp_file handle alive until after the mmap is created
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
}


// Add this helper function

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