use chrono_tz::Tz;
use std::fmt::Debug;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use tokio::sync::RwLock;
use ff_standard_lib::helpers::converters::{convert_to_utc};
use ff_standard_lib::standardized_types::enums::StrategyMode;

pub struct StrategyState {
    pub mode: StrategyMode,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub time_zone: Tz,
    pub warmup_duration: Duration,
    pub is_warmup_complete: RwLock<bool>,
}

impl Debug for StrategyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrategyState")
            .field("mode", &self.mode)
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .field("time_zone", &self.time_zone)
            .field("warmup_duration", &self.warmup_duration)
            .field("is_warmup_complete", &self.is_warmup_complete)
            //.field("statistics", &self.statistics)
            .finish()
    }
}

impl StrategyState {
    pub fn new(mode: StrategyMode, start_date: NaiveDateTime, end_date: NaiveDateTime, time_zone: Tz, warmup_duration: Duration) -> StrategyState {
        let start_date =  convert_to_utc(start_date, time_zone);
        println!("start_date: {:?}", start_date);
        let end_date =  convert_to_utc(end_date, time_zone);
        println!("end_date: {:?}", end_date);
        if start_date > end_date {
            panic!("Start date cannot be greater than end date");
        }
        StrategyState {
            mode,
            start_date,
            end_date,
            time_zone,
            warmup_duration,
            is_warmup_complete: RwLock::new(false),
        }
    }

    pub(crate) async fn set_warmup_complete(&self, is_complete: bool) {
        *self.is_warmup_complete.write().await = is_complete;
    }

    pub async fn is_warmup_complete(&self) -> bool {
        *self.is_warmup_complete.read().await
    }
}



