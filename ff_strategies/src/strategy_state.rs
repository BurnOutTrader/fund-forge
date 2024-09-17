use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use chrono_tz::Tz;
use ff_standard_lib::helpers::converters::convert_to_utc;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use std::fmt::Debug;

#[derive(Clone)]
pub struct StrategyStartState {
    pub mode: StrategyMode,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub time_zone: Tz,
    pub warmup_duration: Duration,
    pub buffer_resolution: Duration,
}

impl Debug for StrategyStartState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrategyState")
            .field("mode", &self.mode)
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .field("time_zone", &self.time_zone)
            .field("warmup_duration", &self.warmup_duration)
            //.field("statistics", &self.statistics)
            .finish()
    }
}

impl StrategyStartState {
    pub fn new(
        mode: StrategyMode,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        time_zone: Tz,
        warmup_duration: Duration,
        buffer_resolution: Option<Duration>,
    ) -> StrategyStartState {
        let buffer_resolution = match buffer_resolution {
            Some(backtest_resolution) => backtest_resolution,
            None => Duration::milliseconds(250),
        };
        let start_date = convert_to_utc(start_date, time_zone);
        println!("start_date: {:?}", start_date);
        let end_date = convert_to_utc(end_date, time_zone);
        println!("end_date: {:?}", end_date);
        if start_date > end_date {
            panic!("Start date cannot be greater than end date");
        }
        StrategyStartState {
            mode,
            start_date,
            end_date,
            time_zone,
            warmup_duration,
            buffer_resolution,
        }
    }
}
