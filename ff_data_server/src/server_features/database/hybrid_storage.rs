use std::cmp::min;
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
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use indicatif::{MultiProgress};
use lazy_static::lazy_static;
use memmap2::{Mmap};
use rust_decimal::Decimal;
use tokio::sync::{OnceCell, Semaphore};
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::interval;
use ff_standard_lib::messages::data_server_messaging::{FundForgeError};
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::resolution::Resolution;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, Symbol, SymbolName};
use crate::{ServerLaunchOptions};

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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use ff_standard_lib::standardized_types::base_data::candle::generate_5_day_candle_data;
    use super::*;


    fn setup_test_storage() -> (HybridStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let options = ServerLaunchOptions {
            data_folder: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = HybridStorage::new(
            Duration::from_secs(3600),  // 1 hour cache duration
            options,
            5,                          // max concurrent downloads
            300                         // update seconds
        );

        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_asof_queries() {
        let (storage, _temp) = setup_test_storage();
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        match storage.save_data_bulk(test_data.clone(), false).await {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error saving test data: {}", e);
                assert!(false);
            }
        }

        let first_candle = test_data[0].time_closed_utc();
        let mid_candle = test_data[12].time_closed_utc();
        let last_candle = test_data.last().unwrap().time_closed_utc();

        let queries = vec![
            // Exact match
            (mid_candle, Some(mid_candle)),
            // Between candles - should get previous
            (mid_candle + chrono::Duration::minutes(30), Some(mid_candle)),
            // Start of next day - should get candle #24
            (test_data[24].time_closed_utc(), Some(test_data[24].time_closed_utc())),
            // Before first candle
            (first_candle - chrono::Duration::hours(1), None),
        ];

        for (query_time, expected_time) in queries {
            let result = storage.get_data_point_asof(
                test_data[0].symbol(),
                &Resolution::Hours(1),
                &BaseDataType::Candles,
                query_time
            ).await.unwrap();

            eprintln!("Query time: {:?}, Expected time: {:?}, Result: {:?}", query_time, expected_time, result);

            match expected_time {
                Some(time) => {
                    assert!(result.is_some(), "Expected data for query time {}", query_time);
                    assert_eq!(result.unwrap().time_closed_utc(), time);
                },
                None => assert!(result.is_none(), "Expected no data for query time {}", query_time)
            }
        }
    }

    #[tokio::test]
    async fn test_five_day_candle_storage() {
        let (storage, _temp) = setup_test_storage();
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        // Save bulk data
        storage.save_data_bulk(test_data.clone(), true).await.unwrap();

        // Get earliest and latest directly from the data
        let expected_earliest = test_data.first().unwrap().time_closed_utc();
        let expected_latest = test_data.last().unwrap().time_closed_utc();

        // Verify earliest and latest times
        let earliest = storage.get_earliest_data_time(
            test_data[0].symbol(),
            &Resolution::Hours(1),
            &BaseDataType::Candles
        ).await.unwrap().unwrap();

        let latest = storage.get_latest_data_time(
            test_data[0].symbol(),
            &Resolution::Hours(1),
            &BaseDataType::Candles
        ).await.unwrap().unwrap();

        assert_eq!(earliest, expected_earliest);
        assert_eq!(latest, expected_latest);
    }

    #[tokio::test]
    async fn test_get_ranges_across_days() {
        let (storage, _temp) = setup_test_storage();
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        storage.save_data_bulk(test_data.clone(), true).await.unwrap();

        // Get actual timestamps from the data
        let day1_start = test_data[0].time_closed_utc();
        let day1_end = test_data[23].time_closed_utc();
        let next_day_start = test_data[24].time_closed_utc();
        let next_day_mid = test_data[36].time_closed_utc();
        let last_day = test_data.last().unwrap().time_closed_utc();

        let ranges = vec![
            // First day only
            (day1_start, day1_end, 24),
            // Span two days
            (day1_end, next_day_mid, 14),  // 12th hour of next day
            // Middle three days
            (next_day_start, last_day, 96),
            // All five days
            (day1_start, last_day, 120)
        ];

        for (start, end, expected_count) in ranges {
            let data = storage.get_data_range(
                test_data[0].symbol(),
                &Resolution::Hours(1),
                &BaseDataType::Candles,
                start,
                end
            ).await.unwrap();

            assert_eq!(data.len(), expected_count,
                       "Expected {} candles between {} and {}",
                       expected_count, start, end);

            // Verify data is sorted
            let mut prev_time = start;
            for point in data {
                let current_time = point.time_closed_utc();
                assert!(current_time >= prev_time, "Data should be sorted by time");
                prev_time = current_time;
            }
        }
    }
}