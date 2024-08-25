use std::collections::BTreeMap;
use std::{fs, io};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use chrono::{Datelike, DateTime, TimeZone, Utc};
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum ReplayControls {
    Play,
    Finished,
    Error,
    Pause,
    AwaitStart,
    Recording
}

/// When a new strategy is registered
/// 1. We decide if the replay goes in backtest or live mode folder.
/// 2. We create the replay folder for the current run using the strategy.owner_id + current time as the folder name.
/// 3. We start adding the strategies to a ReplayEventStream.current_events
/// 4. We record a new file for each month.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct StrategyEventStream {
    current_events: BTreeMap<i64, Vec<StrategyEvent>>,
    file_number: i64,
    folder: String,
    owner_id: OwnerId,
    replay_delay: i64,
    controls: ReplayControls
} 

impl StrategyEventStream {
    pub fn new(strategy_mode: StrategyMode, owner_id: OwnerId) -> StrategyEventStream {
        let base_folder = Path::new("replay");
        let date_time = chrono::Utc::now();
        let time = date_time.format("%Y-%m-%d %H:%M:%S").to_string();
        
        // For backtests we create a new folder by time, for live we load from any existing files.
        let sub_folder = match strategy_mode {
            StrategyMode::Backtest => format!("backtest_{}", time),
            StrategyMode::Live => "live".to_string(),
            StrategyMode::LivePaperTrading => "live_paper_trading".to_string(),
        };
        
        let folder = base_folder.join(owner_id.to_string()).join(time).join(sub_folder);
        
        let mut current_events = BTreeMap::new();
        let mut file_number = 1;
        if !folder.exists() {
            std::fs::create_dir_all(&folder).unwrap();
        }
        else {
            let (number, events) = StrategyEventStream::load_last_events(&folder);
            current_events = events;
            file_number = number;
        }
        
        StrategyEventStream {
            current_events: current_events,
            file_number,
            folder: folder.to_str().unwrap().to_string(),
            owner_id,
            replay_delay: 0,
            controls: ReplayControls::Recording,
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let vec = rkyv::to_bytes::<_, 100000>(self).unwrap();
        vec.into()
    }
    
    pub fn from_bytes(archived: &[u8]) -> Result<StrategyEventStream, FundForgeError> {
        // If the archived bytes do not end with the delimiter, proceed as before
        match rkyv::from_bytes::<StrategyEventStream>(archived) { //Ignore this warning: Trait `Deserialize<UiStreamResponse, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277] 
            Ok(response) => Ok(response),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
    
    /// Saves the current state of the replay stream
    pub fn save_self(&mut self) {
        let folder = Path::new(&self.folder);
        let file_path = folder.join("replay_stream.rkyv");
        let mut file = std::fs::File::create(file_path).unwrap();
        let bytes = self.to_bytes();
        file.write_all(&bytes).unwrap();
    }
    
    /// Loads a replay stream from the folder
    pub fn load_or_none(folder: &Path) -> Option<StrategyEventStream> {
        let file_path = folder.join("replay_stream.rkyv");
        let mut file = std::fs::File::open(file_path).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        match StrategyEventStream::from_bytes(&bytes){
            Ok(stream) => {
                Some(stream)
            },
            Err(e) => {
                println!("Failed to load replay stream: {:?}", e);
                return None;
            }
        }
    }
    
    /// Saves the current strategies to a file
    fn save_events(&mut self) {
        if self.current_events.is_empty() {
            return;
        }
        let file_name = self.file_name(self.file_number);
        let folder = Path::new(&self.folder);
        let file_path = folder.join(file_name);
        let mut file = std::fs::File::create(file_path).unwrap();
        let bytes = rkyv::to_bytes::<_, 10000>(&self.current_events).unwrap();
        file.write_all(&bytes).unwrap();
        
    }

    /// Adds an event to the current strategies
    pub fn add_event(&mut self, time: i64, event: StrategyEvent) {
        let date_time: DateTime<Utc> = Utc.timestamp_opt(time, 0).unwrap();
        // Get the month from the DateTime
        let month = date_time.month();
        let year = date_time.year();

        let last_time = self.current_events.keys().last().unwrap();
        let last_month = Utc.timestamp_opt(*last_time, 0).unwrap().month();
        let last_year = Utc.timestamp_opt(*last_time, 0).unwrap().year();
        if month != last_month || year != last_year {
            self.save_events();
            self.file_number += 1;
            self.current_events = BTreeMap::new();
        }
        if ! self.current_events.contains_key(&time) {
            self.current_events.insert(time, vec![event]);
        }
        else {
            self.current_events.get_mut(&time).unwrap().push(event);
        }
    }
    
    fn file_name(&self, number: i64) -> String {
        format!("events_{}.rkyv", number)
    }

    fn load_last_events(folder_path: &Path) -> (i64, BTreeMap<i64, Vec<StrategyEvent>>) {
        // Check if the directory exists
        if !folder_path.exists() {
            println!("Directory does not exist.");
            return (1, BTreeMap::new())
        }

        // Iterate through the directory
        let mut max_file: Option<PathBuf> = None;
        let mut max_number: i64 = -1;

        if let Ok(entries) = fs::read_dir(folder_path) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if let Some(stripped_name) = file_name.strip_prefix("events_") {
                            if let Ok(number) = stripped_name.trim_end_matches(".rkyv").parse::<i64>() {
                                if number > max_number {
                                    max_number = number;
                                    max_file = Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Open the file with the highest number
        if let Some(file_path) = max_file {
            let mut file = File::open(file_path).expect("Failed to open the file");
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).expect("Failed to read the file");
            let events = rkyv::from_bytes::<BTreeMap<i64, Vec<StrategyEvent>>>(&bytes).unwrap_or_else(|e| {
                println!("Failed to load strategies: {:?}", e);
                BTreeMap::new()
            });
            (max_number, events)
        } else {
            println!("No files found in the directory.");
            (1, BTreeMap::new())
        }
    }

    /// Process replay files in an ordered manner.
    pub async fn replay(&mut self) {
        let folder = Path::new(&self.folder);
        if let Ok(sorted_files) = sort_files_by_name(folder) {
            for path in sorted_files {
                let mut file = match File::open(&path) {
                    Ok(file) => file,
                    Err(e) => {
                        eprintln!("Failed to open file {:?}: {}", path, e);
                        continue;
                    }
                };

                let mut bytes = Vec::new();
                if let Err(e) = file.read_to_end(&mut bytes) {
                    eprintln!("Failed to read file {:?}: {}", path, e);
                    continue;
                }

                let events = match rkyv::from_bytes::<BTreeMap<i64, Vec<StrategyEvent>>>(&bytes) {
                    Ok(events) => events,
                    Err(e) => {
                        eprintln!("Failed to deserialize strategies from {:?}: {}", path, e);
                        continue;
                    }
                };

                //replay the histocial strategies
                if !events.is_empty() {
                    for (_, event_list) in events {
                        self.process_events(&event_list).await;
                    }
                }
            }
            //replay the current strategies not yet saved
            if !self.current_events.is_empty() {
                for (_, event_list) in &self.current_events {
                    self.process_events(event_list).await;
                }
            }
        } else {
            eprintln!("Failed to sort files in the folder {}", self.folder);
        }
    }
    
    async fn process_events(&self, events: &Vec<StrategyEvent>) {
       println!("{:?}", events);
    }
}

/// Sort files by numerical order extracted from their names.
fn sort_files_by_name(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<_> = fs::read_dir(folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext == "rkyv"))
        .collect();

    files.sort_by_key(|path| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|s| s.strip_prefix("events_"))
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0)
    });

    Ok(files)
}
