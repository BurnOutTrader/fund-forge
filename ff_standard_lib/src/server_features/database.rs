use std::collections::{BTreeMap, HashMap};
use crate::standardized_types::base_data::base_data_enum::BaseDataEnum;
use std::path::{Path, PathBuf};
use std::fs::{read_dir, File, OpenOptions};
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use futures::future;
use memmap2::{Mmap, MmapMut};
use crate::messages::data_server_messaging::{FundForgeError};
use crate::standardized_types::base_data::base_data_type::BaseDataType;
use crate::standardized_types::base_data::traits::BaseData;
use crate::standardized_types::resolution::Resolution;
use crate::standardized_types::subscriptions::{DataSubscription, Symbol};
use crate::standardized_types::time_slices::TimeSlice;

// Todo, save as 1 file per week.
//todo add update management handling, so we dont access data while it is being updated. instead we join a que to await the updates
// use market type in folder structure
pub struct HybridStorage {
    base_path: PathBuf,
    mmap_cache: Arc<Mutex<HashMap<String, Arc<Mmap>>>>,
}

impl HybridStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            mmap_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_file_path(&self, symbol: &Symbol, resolution: &str, data_type: &BaseDataType, date: &DateTime<Utc>) -> PathBuf {
        self.base_path
            .join(symbol.data_vendor.to_string())
            .join(symbol.market_type.to_string())
            .join(symbol.name.to_string())
            .join(resolution)
            .join(data_type.to_string())
            .join(format!("{:04}{:02}.bin", date.year(), date.month()))
    }

    pub async fn save_data(&self, data: &BaseDataEnum) -> io::Result<()> {
        if !data.is_closed() {
            return Ok(());
        }

        let file_path = self.get_file_path(
            data.symbol(),
            &data.resolution().to_string(),
            &data.base_data_type(),
            &data.time_closed_utc()
        );
        self.save_data_to_file(&file_path, &[data.clone()]).await
    }

    pub async fn save_data_bulk(&self, data: Vec<BaseDataEnum>) -> io::Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let mut grouped_data: HashMap<(Symbol, Resolution, BaseDataType, i32, u32), Vec<BaseDataEnum>> = HashMap::new();

        for d in data {
            let key = (d.symbol().clone(), d.resolution(), d.base_data_type(), d.time_closed_utc().year(), d.time_closed_utc().month());
            grouped_data.entry(key).or_insert_with(Vec::new).push(d);
        }

        for ((symbol, resolution, data_type, year, month), group) in grouped_data {
            let date: DateTime<Utc> = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
            let file_path = self.get_file_path(&symbol, &resolution.to_string(), &data_type, &date);
            self.save_data_to_file(&file_path, &group).await?;
        }

        Ok(())
    }

    async fn save_data_to_file(&self, file_path: &Path, new_data: &[BaseDataEnum]) -> io::Result<()> {
        std::fs::create_dir_all(file_path.parent().unwrap())?;

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

        let mut cache = self.mmap_cache.lock().unwrap();
        let mmap = unsafe { MmapMut::map_mut(&file)? }.make_read_only()?;
        cache.insert(file_path.to_string_lossy().to_string(), mmap.into());

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

        let mut current_date: DateTime<Utc> = Utc
            .with_ymd_and_hms(start.year(), start.month(), start.day(), 0, 0, 0)
            .unwrap();

        while current_date <= end {
            let file_path = self.get_file_path(symbol, &resolution.to_string(), data_type, &current_date);
            if let Ok(mmap) = self.get_or_create_mmap(&file_path) {
                let month_data = BaseDataEnum::from_array_bytes(&mmap[..].to_vec()).unwrap(); //todo handle instead of unwrap
                all_data.extend(month_data.into_iter().filter(|d| d.time_closed_utc() >= start && d.time_closed_utc() <= end));
            }
            current_date = current_date.with_month(current_date.month() + 1).unwrap_or_else(|| current_date.with_year(current_date.year() + 1).unwrap().with_month(1).unwrap());
        }

        Ok(all_data)
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

    pub async fn get_latest_data_point(
        &self,
        symbol: &Symbol,
        resolution: &str,
        data_type: &BaseDataType,
    ) -> Result<Option<BaseDataEnum>, Box<dyn std::error::Error>> {
        let current_date = Utc::now();
        let mut date = current_date;

        for _ in 0..36 {  // Check up to 36 months back
            let file_path = self.get_file_path(symbol, resolution, data_type, &date);
            if let Ok(mmap) = self.get_or_create_mmap(&file_path) {
                let month_data = BaseDataEnum::from_array_bytes(&mmap.to_vec())?;
                if let Some(latest) = month_data.into_iter().max_by_key(|d| d.time_closed_utc()) {
                    return Ok(Some(latest));
                }
            }
            date = date.with_month(date.month() - 1).unwrap_or_else(|| date.with_year(date.year() - 1).unwrap().with_month(12).unwrap());
        }

        Ok(None)
    }

    pub async fn get_latest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &str,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        let base_path = self.base_path
            .join(symbol.data_vendor.to_string())
            .join(symbol.name.to_string())
            .join(resolution)
            .join(data_type.to_string());

        let mut latest_file: Option<PathBuf> = None;
        let mut latest_modified: Option<std::time::SystemTime> = None;

        if let Ok(entries) = read_dir(&base_path) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if latest_modified.map_or(true, |t| modified > t) {
                                latest_modified = Some(modified);
                                latest_file = Some(path);
                            }
                        }
                    }
                }
            }
        }

        if let Some(file_path) = latest_file {
            if let Ok(mmap) = self.get_or_create_mmap(&file_path) {
                if let Ok(month_data) = BaseDataEnum::from_array_bytes(&mmap.to_vec()) {
                    return Ok(month_data.into_iter().map(|d| d.time_closed_utc()).max());
                }
            }
        }

        Ok(None)
    }

    pub async fn get_earliest_data_time(
        &self,
        symbol: &Symbol,
        resolution: &str,
        data_type: &BaseDataType,
    ) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
        let base_path = self.base_path
            .join(symbol.data_vendor.to_string())
            .join(symbol.name.to_string())
            .join(resolution)
            .join(data_type.to_string());

        let mut earliest_file: Option<PathBuf> = None;
        let mut earliest_modified: Option<std::time::SystemTime> = None;

        if let Ok(entries) = read_dir(&base_path) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if earliest_modified.map_or(true, |t| modified < t) {
                                earliest_modified = Some(modified);
                                earliest_file = Some(path);
                            }
                        }
                    }
                }
            }
        }

        if let Some(file_path) = earliest_file {
            if let Ok(mmap) = self.get_or_create_mmap(&file_path) {
                if let Ok(month_data) = BaseDataEnum::from_array_bytes(&mmap.to_vec()) {
                    return Ok(month_data.into_iter().map(|d| d.time_closed_utc()).min());
                }
            }
        }

        Ok(None)
    }

    fn get_or_create_mmap(&self, file_path: &Path) -> io::Result<Arc<Mmap>> {
        let mut cache = self.mmap_cache.lock().unwrap();
        if let Some(mmap) = cache.get(file_path.to_string_lossy().as_ref()) {
            Ok(Arc::clone(mmap))
        } else {
            let file = File::open(file_path)?;
            let mmap = Arc::new(unsafe { Mmap::map(&file)? });
            cache.insert(file_path.to_string_lossy().to_string(), Arc::clone(&mmap));
            Ok(mmap)
        }
    }
}