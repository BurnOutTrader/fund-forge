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
use futures_util::future::join_all;
use indicatif::{MultiProgress};
use lazy_static::lazy_static;
use memmap2::{Mmap};
use tokio::sync::{OnceCell, Semaphore};
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::{interval, timeout};
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
    file_locks: Arc<DashMap<String, Arc<Semaphore>>>,
    pub(crate) download_tasks: Arc<DashMap<(SymbolName, BaseDataType, Resolution), JoinHandle<()>>>,
    pub(crate) options: ServerLaunchOptions,
    pub(crate) download_semaphore: Arc<Semaphore>,
    pub(crate) update_seconds: u64,
}

impl HybridStorage {
    pub fn new(clear_cache_duration: Duration, options: ServerLaunchOptions, max_concurrent_downloads: usize, update_seconds: u64) -> Self {
        let max_concurrent_downloads= min(max_concurrent_downloads, 35);
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
                    let mut remove_semaphore = false;
                    if let Some(file_semaphore) = file_locks.get(&path) {
                        if file_semaphore.available_permits() == 1 {
                            remove_semaphore = true;
                        }
                    }
                    mmap_cache.remove(&path);
                    if let Some((_, mmap)) = mmap_cache.remove(&path) {
                        drop(mmap); // Explicitly drop the mmap
                    }
                    cache_last_accessed.remove(&path);
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
                d.time_closed_utc().date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap()
            );
            grouped_data.entry(key).or_insert_with(Vec::new).push(d);
        }

        //println!("Grouped data into {} files", grouped_data.len());

        for ((symbol, resolution, data_type, date), group) in grouped_data {
            let file_path = self.get_file_path(&symbol, &resolution, &data_type, &date, true);
            //println!("Saving {} data points to file: {:?}", group.len(), file_path);
            self.save_data_to_file(&file_path, &group).await?;
        }

        Ok(())
    }

    pub(crate) async fn get_or_create_mmap(&self, file_path: &Path, resolution: Resolution) -> io::Result<Arc<Mmap>> {
        let path_str = file_path.to_string_lossy().to_string();

        if let Some(mmap) = self.mmap_cache.get(&path_str) {
            self.cache_last_accessed.insert(path_str.clone(), Utc::now());
            println!("Found mmap in cache for file: {:?}", file_path);
            return Ok(Arc::clone(mmap.value()));
        }

        // Handle cache eviction
        let oldest_path = if self.mmap_cache.len() >= 200 {
            self.cache_last_accessed
                .iter()
                .min_by_key(|entry| entry.value().clone())
                .map(|entry| entry.key().clone())
        } else {
            None
        };

        if let Some(oldest_path) = oldest_path {
            if let Some(semaphore) = self.file_locks.get(&oldest_path) {
                let permit = semaphore.value().acquire().await.map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error acquiring lock for cache cleanup: {}", e)
                    )
                })?;

                if let Some((_, mmap)) = self.mmap_cache.remove(&file_path.to_string_lossy().to_string()) {
                    drop(mmap);
                }
                self.cache_last_accessed.remove(&oldest_path);
                drop(permit);
            }
            self.file_locks.remove(&oldest_path);
        }

        let semaphore = self.file_locks.entry(file_path.to_str().unwrap().to_string()).or_insert(Arc::new(Semaphore::new(1)));
        let _permit = match semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                panic!("Thread Tripwire, Error acquiring save permit: {}", e)
            }
        };

        let mut file = File::open(file_path)?;
        let file_size = file.metadata()?.len() as usize;

        // Pre-allocate for compressed data
        let mut compressed_data = Vec::new();
        compressed_data.try_reserve(file_size)
            .map_err(|e| io::Error::new(
                io::ErrorKind::OutOfMemory,
                format!("Failed to allocate memory for compressed data: {}", e)
            ))?;

        const MB: usize = 1024 * 1024;
        let buffer_size = match resolution {
            Resolution::Minutes(_) => 1 * MB,
            Resolution::Hours(_) => 1 * MB,
            Resolution::Instant => 20 * MB,
            Resolution::Ticks(_) => 13 * MB,
            Resolution::Seconds(_) => 5 * MB,
            _ => 1 * MB
        };
        // Create buffer on heap instead of stack
        let mut buffer = vec![0; buffer_size];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    compressed_data.try_reserve(n)
                        .map_err(|e| io::Error::new(
                            io::ErrorKind::OutOfMemory,
                            format!("Failed to allocate during read: {}", e)
                        ))?;
                    compressed_data.extend_from_slice(&buffer[..n]);
                }
                Err(e) => return Err(e),
            }
        }

        // Pre-allocate decompressed data - estimate size based on compression ratio
        let mut decompressed = Vec::new();
        let estimated_size = compressed_data.len() * 3; // Assuming ~3:1 compression ratio
        decompressed.try_reserve(estimated_size)
            .map_err(|e| io::Error::new(
                io::ErrorKind::OutOfMemory,
                format!("Failed to allocate memory for decompressed data: {}", e)
            ))?;

        // Decompress in chunks
        let cursor = std::io::Cursor::new(&compressed_data);
        let buf_reader = std::io::BufReader::new(cursor);
        let mut decoder = GzDecoder::new(buf_reader);
        loop {
            match decoder.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    decompressed.try_reserve(n)
                        .map_err(|e| io::Error::new(
                            io::ErrorKind::OutOfMemory,
                            format!("Failed to allocate during decompression: {}", e)
                        ))?;
                    decompressed.extend_from_slice(&buffer[..n]);
                }
                Err(e) => return Err(e),
            }
        }

        // Create temporary file
        let temp_path = file_path.with_extension("tmp");
        {
            let mut temp_file = File::create(&temp_path)?;
            // Write in chunks to avoid large buffer allocations
            for chunk in decompressed.chunks(8192) {
                temp_file.write_all(chunk)?;
            }
            temp_file.sync_all()?;
        }

        // Create mmap
        let temp_file = File::open(&temp_path)?;
        let mmap = Arc::new(unsafe { Mmap::map(&temp_file)? });
        self.mmap_cache.insert(path_str.clone(), Arc::clone(&mmap));
        self.cache_last_accessed.insert(path_str.clone(), Utc::now());

        drop(temp_file);
        std::fs::remove_file(&temp_path)?;
        //println!("Created mmap for file: {:?}", file_path);
        Ok(mmap)
    }


    /// This first updates the file on disk, then the file in memory is replaced with the new file, therefore we do not have saftey issues.
    async fn save_data_to_file(&self, file_path: &Path, new_data: &[BaseDataEnum]) -> io::Result<()> {
        if new_data.is_empty() {
            return Ok(())
        }

        let semaphore = self.file_locks.entry(file_path.to_str().unwrap().to_string()).or_insert(Arc::new(Semaphore::new(1)));
        let _permit = match semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                panic!("Thread Tripwire, Error acquiring save permit: {}", e)
            }
        };

        if let Some((_, mmap)) = self.mmap_cache.remove(&file_path.to_string_lossy().to_string()) {
            drop(mmap);
            self.cache_last_accessed.remove(&file_path.to_string_lossy().to_string());
        }

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(file_path)?;

        // Read and decompress existing data
        let mut compressed_data = Vec::new();
        file.read_to_end(&mut compressed_data)?;

        let existing_data = if !compressed_data.is_empty() {
            const MB: usize = 1024 * 1024;
            let buffer_size = match new_data.get(0).unwrap().resolution() {
                Resolution::Minutes(_) => 1 * MB,
                Resolution::Hours(_) => 1 * MB,
                Resolution::Instant => 20 * MB,
                Resolution::Ticks(_) => 13 * MB,
                Resolution::Seconds(_) => 5 * MB,
                _ => 1 * MB
            };
            let mut buffer = vec![0; buffer_size];

            let data_len = compressed_data.len();
            let cursor = std::io::Cursor::new(compressed_data);
            let mut decoder = GzDecoder::new(cursor);
            let mut decompressed = Vec::new();

            // Assuming ~3:1 compression ratio for initial allocation
            decompressed.try_reserve(data_len * 98)
                .map_err(|e| io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    format!("Failed to allocate memory for decompression: {}", e)
                ))?;

            loop {
                match decoder.read(&mut buffer[..]) {
                    Ok(0) => break,
                    Ok(n) => {
                        decompressed.try_reserve(n)
                            .map_err(|e| io::Error::new(
                                io::ErrorKind::OutOfMemory,
                                format!("Failed to allocate during decompression: {}", e)
                            ))?;
                        decompressed.extend_from_slice(&buffer[..n]);
                    }
                    Err(e) => return Err(e),
                }
            }

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
        // Compress the serialized data using a Cursor
        let mut compressed_buffer = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut compressed_buffer);
            let mut encoder = GzEncoder::new(cursor, Compression::default());
            encoder.write_all(&bytes)?;
            encoder.finish()?; // Ensure compression is completed
        }

        // Write to file
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;

        match file.write_all(&compressed_buffer) {
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
        Ok(())
    }

    pub async fn get_files_in_range (
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

    // Optionally, we could also make this more efficient by using parallel reads with a bounded semaphore:
    pub async fn get_compressed_files_in_range(
        &self,
        subscription: Vec<DataSubscription>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Vec<u8>>, FundForgeError> {
        const TIMEOUT_DURATION: Duration = Duration::from_secs(60);
        const MAX_CONCURRENT_FILES: usize = 10;

        if subscription.is_empty() {
            return Err(FundForgeError::ClientSideErrorDebug("No subscriptions provided for file range".to_string()));
        }

        // Create a bounded semaphore for concurrent file operations
       // println!("Creating file semaphore");
        let file_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FILES));
        let mut file_tasks = Vec::new();

        for subscription in subscription {
            let file_paths = self.get_files_in_range(
                &subscription.symbol,
                &subscription.resolution,
                &subscription.base_data_type,
                start,
                end
            ).await?;

            for file_path in file_paths {
                if !file_path.exists() {
                    continue;
                }
                //println!("Adding file task for {:?}", file_path);

                let path_str = file_path.to_string_lossy().to_string();
                let file_lock = self.file_locks
                    .entry(path_str.clone())
                    .or_insert_with(|| Arc::new(Semaphore::new(1)))
                    .clone();

                let file_sem = file_semaphore.clone();

                // Create an async task for each file read
                let task = async move {
                    // Acquire the global file semaphore
                    let _file_permit = match file_sem.acquire().await {
                        Ok(p) => p,
                        Err(e) => return Err(FundForgeError::ServerErrorDebug(
                            format!("Error acquiring global file semaphore: {}", e)
                        )),
                    };

                    // Acquire the file-specific lock with timeout
                    //println!("Acquiring file lock for {:?}", file_path);
                    let file_permit = match timeout(
                        Duration::from_secs(60),
                        file_lock.acquire()
                    ).await {
                        Ok(permit) => match permit {
                            Ok(p) => p,
                            Err(e) => return Err(FundForgeError::ServerErrorDebug(
                                format!("Error acquiring file lock: {}", e)
                            )),
                        }
                        Err(_) => return Err(FundForgeError::ServerErrorDebug(
                            format!("Timeout waiting for file lock: {}", path_str)
                        )),
                    };

                    //println!("Reading file {:?}", file_path);
                    // Open and read the file with timeout
                    let result = timeout(
                        Duration::from_secs(60),
                        async {
                            let mut file = File::open(&file_path)
                                .map_err(|e| FundForgeError::ServerErrorDebug(
                                    format!("Error opening file {:?}: {}", file_path, e)
                                ))?;

                            let file_size = file.metadata()
                                .map_err(|e| FundForgeError::ServerErrorDebug(
                                    format!("Error getting file size {:?}: {}", file_path, e)
                                ))?.len() as usize;
                            println!("File size: {}", file_size);

                            let mut compressed_data = Vec::with_capacity(file_size);
                            file.read_to_end(&mut compressed_data)
                                .map_err(|e| FundForgeError::ServerErrorDebug(
                                    format!("Error reading file {:?}: {}", file_path, e)
                                ))?;

                            Ok::<Vec<u8>, FundForgeError>(compressed_data)
                        }
                    ).await;

                    //println!("Releasing file lock for {:?}", file_path);
                    drop(file_permit);

                    //println!("Releasing global file semaphore");
                    match result {
                        Ok(data) => data,
                        Err(_) => Err(FundForgeError::ServerErrorDebug(
                            format!("Timeout reading file: {}", path_str)
                        )),
                    }
                };

                file_tasks.push(task);
            }
        }

        //println!("Executing file tasks");
        // Execute all file tasks with an overall timeout
        let results = match timeout(TIMEOUT_DURATION, join_all(file_tasks)).await {
            Ok(results) => results,
            Err(_) => return Err(FundForgeError::ServerErrorDebug(
                "Overall operation timeout exceeded".to_string()
            )),
        };

        // Collect successful results
        //println!("Processing file results");
        let mut files_data = Vec::new();
        for result in results {
            match result {
                Ok(data) => files_data.push(data),
                Err(e) => eprintln!("Error processing file: {}", e),
            }
        }

        //println!("Returning {} files", files_data.len());
        if files_data.is_empty() {
            return Err(FundForgeError::ServerErrorDebug("No files found in range".to_string()));
        }

        Ok(files_data)
    }
}

#[cfg(test)]
mod test {
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
    async fn test_five_day_candle_storage() {
        let (storage, _temp) = setup_test_storage();
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        // Save bulk data
        storage.save_data_bulk(test_data.clone()).await.unwrap();

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

        storage.save_data_bulk(test_data.clone()).await.unwrap();

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

    #[tokio::test]
    async fn test_get_latest_data_time_repeated() {
        use std::time::Duration;
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        for iteration in 1..=3 {
            println!("Running iteration {}", iteration);

            let (storage, _temp) = setup_test_storage();

            // Generate test data
            let mut test_data = generate_5_day_candle_data().iter()
                .map(|c| BaseDataEnum::Candle(c.clone()))
                .collect::<Vec<_>>();

            // Randomly shuffle the data to ensure order doesn't matter
            let mut rng = thread_rng();
            test_data.shuffle(&mut rng);

            // Save bulk data to storage
            storage.save_data_bulk(test_data.clone()).await.unwrap();

            // Add a small delay to ensure data is fully saved
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Expected latest timestamp from test data
            let expected_latest = test_data.iter()
                .map(|d| d.time_closed_utc())
                .max()
                .unwrap();

            // Call `get_latest_data_time` multiple times to verify consistency
            for _ in 0..3 {
                let latest_time = storage.get_latest_data_time(
                    test_data[0].symbol(),
                    &Resolution::Hours(1),
                    &BaseDataType::Candles
                ).await.unwrap();

                assert_eq!(
                    latest_time,
                    Some(expected_latest),
                    "Iteration {}, Expected latest time: {}, but got: {:?}",
                    iteration,
                    expected_latest,
                    latest_time
                );

                // Add a small delay between checks
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            // Clean up between iterations
            drop(storage);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    async fn test_get_earliest_data_time_repeated() {
        use std::time::Duration;
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        for iteration in 1..=3 {
            println!("Running iteration {}", iteration);

            let (storage, _temp) = setup_test_storage();

            // Generate test data
            let mut test_data = generate_5_day_candle_data().iter()
                .map(|c| BaseDataEnum::Candle(c.clone()))
                .collect::<Vec<_>>();

            // Randomly shuffle the data to ensure order doesn't matter
            let mut rng = thread_rng();
            test_data.shuffle(&mut rng);

            // Save bulk data to storage
            storage.save_data_bulk(test_data.clone()).await.unwrap();

            // Add a small delay to ensure data is fully saved
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Expected earliest timestamp from test data
            let expected_earliest = test_data.iter()
                .map(|d| d.time_closed_utc())
                .min()
                .unwrap();

            // Call `get_earliest_data_time` multiple times to verify consistency
            for check_num in 1..=3 {
                let earliest_time = storage.get_earliest_data_time(
                    test_data[0].symbol(),
                    &Resolution::Hours(1),
                    &BaseDataType::Candles
                ).await.unwrap();

                assert_eq!(
                    earliest_time,
                    Some(expected_earliest),
                    "Iteration {}, Check {}: Expected earliest time: {}, but got: {:?}",
                    iteration,
                    check_num,
                    expected_earliest,
                    earliest_time
                );

                // Add a small delay between checks
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            // Verify that earliest and latest times are correct relative to each other
            let latest_time = storage.get_latest_data_time(
                test_data[0].symbol(),
                &Resolution::Hours(1),
                &BaseDataType::Candles
            ).await.unwrap();

            assert!(
                latest_time.unwrap() > expected_earliest,
                "Iteration {}: Latest time should be after earliest time. Latest: {:?}, Earliest: {}",
                iteration,
                latest_time,
                expected_earliest
            );

            // Clean up between iterations
            drop(storage);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    async fn test_get_data_range() {
        let (storage, _temp) = setup_test_storage();
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        // Save bulk data to storage
        storage.save_data_bulk(test_data.clone()).await.unwrap();

        // Define test ranges
        let day1_start = test_data.first().unwrap().time_closed_utc();
        let day1_end = test_data[23].time_closed_utc(); // Last candle of day 1
        let next_day_start = test_data[24].time_closed_utc(); // First candle of day 2
        let next_day_mid = test_data[36].time_closed_utc(); // Middle of day 2
        let last_day = test_data.last().unwrap().time_closed_utc();

        // Adjust expected counts based on 1-hour resolution
        let ranges = vec![
            (day1_start, day1_end, 24),                      // First day only
            (day1_start, next_day_mid, 37),                 // Day 1 start to middle of day 2
            (next_day_start, last_day, test_data.len() - 24), // Day 2 start to last candle
            (day1_start, last_day, test_data.len()),        // Entire range
        ];

        for (start, end, expected_count) in ranges {
            let data = storage.get_data_range(
                test_data[0].symbol(),
                &Resolution::Hours(1),
                &BaseDataType::Candles,
                start,
                end
            ).await.unwrap();

            assert_eq!(
                data.len(),
                expected_count,
                "Expected {} candles between {} and {}, but got {}",
                expected_count,
                start,
                end,
                data.len()
            );

            // Ensure data is sorted and within the range
            assert!(data.iter().all(|d| d.time_closed_utc() >= start && d.time_closed_utc() <= end),
                    "Data contains timestamps outside the specified range.");
            assert!(data.windows(2).all(|w| w[0].time_closed_utc() <= w[1].time_closed_utc()),
                    "Data is not sorted by time.");
        }
    }

    #[tokio::test]
    async fn test_get_data_point_asof() {
        let (storage, _temp) = setup_test_storage();
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        // Save bulk data to storage
        storage.save_data_bulk(test_data.clone()).await.unwrap();

        // Define test cases
        let target_times = vec![
            test_data[5].time_closed_utc(), // Specific point in day 1
            test_data[23].time_closed_utc(), // Last point in day 1
            test_data[24].time_closed_utc(), // First point in day 2
            test_data[36].time_closed_utc(), // Middle of day 2
            test_data.last().unwrap().time_closed_utc(), // Last point in the range
            test_data[5].time_closed_utc() - chrono::Duration::minutes(30), // No exact match, look earlier
        ];

        for target_time in target_times {
            // Expected result: the latest data point <= target_time
            let expected_point = test_data
                .iter()
                .filter(|d| d.time_closed_utc() <= target_time)
                .max_by_key(|d| d.time_closed_utc())
                .cloned();

            // Call get_data_point_asof
            let result = storage.get_data_point_asof(
                test_data[0].symbol(),
                &Resolution::Hours(1),
                &BaseDataType::Candles,
                target_time
            ).await.unwrap();

            // Validate result
            assert_eq!(
                result,
                expected_point,
                "For target time {}: expected {:?}, got {:?}",
                target_time,
                expected_point,
                result
            );
        }
    }
    #[tokio::test]
    async fn test_multiple_strategy_connections() {
        let (storage, _temp) = setup_test_storage();

        // Generate initial test data and ensure proper serialization
        let test_data = generate_5_day_candle_data().iter()
            .map(|c| BaseDataEnum::Candle(c.clone()))
            .collect::<Vec<_>>();

        // Save each day's data separately to ensure proper file structure
        for day_data in test_data.chunks(24) {
            if day_data.is_empty() {
                continue;
            }

            println!("Saving chunk starting at {}", day_data[0].time_closed_utc());

            // Verify the data before saving
            let bytes = BaseDataEnum::vec_to_bytes(day_data.to_vec());
            // Verify we can deserialize what we're about to save
            let _verify = BaseDataEnum::from_array_bytes(&bytes)
                .expect("Data should be deserializable before saving");

            // Save with verification
            let save_result = storage.save_data_bulk(day_data.to_vec()).await;
            assert!(save_result.is_ok(), "Failed to save data: {:?}", save_result);

            // Immediately verify the saved data
            let file_path = storage.get_file_path(
                day_data[0].symbol(),
                &day_data[0].resolution(),
                &day_data[0].base_data_type(),
                &day_data[0].time_closed_utc(),
                false
            );
            println!("Saved to file: {:?}", file_path);
            assert!(file_path.exists(), "File was not created");
        }

        let symbol = test_data[0].symbol();
        let resolution = Resolution::Hours(1);
        let data_type = BaseDataType::Candles;

        // Verify we can read the data back
        let initial_latest = storage.get_latest_data_time(
            symbol,
            &resolution,
            &data_type
        ).await.unwrap();

        println!("Initial latest time: {:?}", initial_latest);
        assert!(initial_latest.is_some(), "Failed to get latest time");

        // First strategy connection simulation
        let start_time = test_data[0].time_closed_utc();
        let end_time = test_data.last().unwrap().time_closed_utc();
        println!("Fetching data range: {} to {}", start_time, end_time);

        let data1 = storage.get_data_range(
            symbol,
            &resolution,
            &data_type,
            start_time,
            end_time
        ).await.unwrap();

        println!("First fetch returned {} data points", data1.len());
        assert!(!data1.is_empty(), "First data fetch returned empty set");

        // Verify data content
        if let Some(first) = data1.first() {
            println!("First data point: {}", first.time_closed_utc());
        }
        if let Some(last) = data1.last() {
            println!("Last data point: {}", last.time_closed_utc());
        }

        // Simulate second strategy connection with earlier start
        let early_start = start_time - chrono::Duration::days(5);
        println!("Second fetch range: {} to {}", early_start, end_time);

        let data2 = storage.get_data_range(
            symbol,
            &resolution,
            &data_type,
            early_start,
            end_time
        ).await.unwrap();

        println!("Second fetch returned {} data points", data2.len());
        assert!(!data2.is_empty(), "Second data fetch returned empty set");

        // Compare results
        assert_eq!(
            data1.last().unwrap().time_closed_utc(),
            data2.last().unwrap().time_closed_utc(),
            "Latest times don't match between fetches"
        );

        // Verify file contents directly
        let files = storage.get_files_in_range(
            symbol,
            &resolution,
            &data_type,
            start_time,
            end_time
        ).await.unwrap();

        println!("Found {} files in range", files.len());
        for file in files {
            println!("Verifying file: {:?}", file);
            assert!(file.exists(), "File doesn't exist: {:?}", file);
            let file_size = std::fs::metadata(&file).unwrap().len();
            assert!(file_size > 0, "File is empty: {:?}", file);
        }
    }
}