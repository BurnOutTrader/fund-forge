use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::{interval, sleep, Duration as TokioDuration};
use crate::strategies::strategy_events::StrategyEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventTimeEnum {
    /// Events to occur at on a specific day of the week
    Weekday { day: Weekday, fire_in_warmup: bool },
    /// Events to occur at a specific hour of the day
    HourOfDay { hour: u32, fire_in_warmup: bool },
    /// Events to occur at a specific time on a specific day of the week
    TimeOnWeekDay {
        day: Weekday,
        hour: u32,
        minute: u32,
        second: u32,
        fire_in_warmup: bool,
    },
    /// Events to occur at a specific date and time only once
    DateTime {
        date_time: DateTime<Utc>,
        fire_in_warmup: bool,
    },
    /// Events to occur at a specific time of the day
    TimeOfDay {
        hour: u32,
        minute: u32,
        second: u32,
        fire_in_warmup: bool,
    },
    /// Events to occur at a specific interval
    Every {
        duration: Duration,
        next_time: DateTime<Utc>,
        fire_in_warmup: bool,
    },
}

impl EventTimeEnum {
    pub fn event_time(&self, current_time: DateTime<Utc>) -> bool {
        match self {
            EventTimeEnum::Weekday { day, .. } => {
                if current_time.weekday() == *day {
                    return true;
                }
            }
            EventTimeEnum::HourOfDay { hour, .. } => {
                if current_time.hour() == *hour {
                    return true;
                }
            }
            EventTimeEnum::TimeOnWeekDay {
                day,
                hour,
                minute,
                second,
                ..
            } => {
                if current_time.weekday() == *day
                    && current_time.hour() == *hour
                    && current_time.minute() == *minute
                    && current_time.second() == *second
                {
                    return true;
                }
            }
            EventTimeEnum::DateTime { date_time, .. } => {
                if current_time == *date_time {
                    return true;
                }
            }
            EventTimeEnum::TimeOfDay {
                hour,
                minute,
                second,
                ..
            } => {
                if current_time.hour() == *hour
                    && current_time.minute() == *minute
                    && current_time.second() == *second
                {
                    return true;
                }
            }
            EventTimeEnum::Every { next_time, .. } => {
                if current_time == *next_time {
                    return true;
                }
            }
        }
        false
    }

    pub fn fire_in_warmup(&self) -> bool {
        match self {
            EventTimeEnum::Weekday { fire_in_warmup, .. } => fire_in_warmup.clone(),
            EventTimeEnum::HourOfDay { fire_in_warmup, .. } => fire_in_warmup.clone(),
            EventTimeEnum::TimeOnWeekDay { fire_in_warmup, .. } => fire_in_warmup.clone(),
            EventTimeEnum::DateTime { fire_in_warmup, .. } => fire_in_warmup.clone(),
            EventTimeEnum::TimeOfDay { fire_in_warmup, .. } => fire_in_warmup.clone(),
            EventTimeEnum::Every { fire_in_warmup, .. } => fire_in_warmup.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimedEvent {
    name: String,
    time: EventTimeEnum,
}

impl TimedEvent {
    pub fn new(name: String, event_time: EventTimeEnum) -> Self {
        TimedEvent {
            name,
            time: event_time,
        }
    }
}

pub struct TimedEventHandler {
    pub(crate) schedule: Arc<RwLock<Vec<TimedEvent>>>,
    last_fired: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    strategy_event_sender: Sender<StrategyEvent>
}

impl TimedEventHandler {
    pub fn new(strategy_event_sender: Sender<StrategyEvent>) -> Self {
        TimedEventHandler {
            schedule: Default::default(),
            last_fired: Arc::new(RwLock::new(HashMap::new())),
            strategy_event_sender
        }
    }

    pub async fn add_event(&self, scheduled_event: TimedEvent) {
        self.schedule.write().await.push(scheduled_event);
    }

    pub async fn remove_event(&self, name: String) {
        self.schedule
            .write()
            .await
            .retain(|event| event.name != name);
        self.last_fired.write().await.remove(&name);
    }

    pub async fn run_time_updates(self: Arc<Self>) {
        task::spawn(async move {
            // Wait until the next whole second
            let now = Utc::now();
            let sleep_duration = TokioDuration::from_nanos(1_000_000_000 - now.timestamp_subsec_nanos() as u64);
            sleep(sleep_duration).await;

            // Create an interval that ticks every second
            let mut interval = interval(TokioDuration::from_secs(1));

            // Run indefinitely
            loop {
                // Wait for the next tick
                interval.tick().await;

                // Get current time
                let current_time = Utc::now();

                // Run the update
                self.update_time(current_time).await;
            }
        });
    }

    pub async fn update_time(&self, current_time: DateTime<Utc>) {
        let mut schedule = self.schedule.write().await;
        let mut last_fired = self.last_fired.write().await;
        if schedule.is_empty() {
            return;
        }
        let mut events_to_remove = vec![];
        for event in schedule.iter_mut() {
            if event.time.event_time(current_time) {
                let should_fire = match &event.time {
                    EventTimeEnum::Weekday { .. } => {
                        last_fired.get(&event.name).map_or(true, |&last| last.date_naive() < current_time.date_naive())
                    },
                    EventTimeEnum::HourOfDay { .. } => {
                        last_fired.get(&event.name).map_or(true, |&last|
                            last.date_naive() < current_time.date_naive() || last.hour() < current_time.hour()
                        )
                    },
                    EventTimeEnum::TimeOnWeekDay { .. } | EventTimeEnum::TimeOfDay { .. } => {
                        last_fired.get(&event.name).map_or(true, |&last| last.date_naive() < current_time.date_naive())
                    },
                    EventTimeEnum::DateTime { .. } => true,
                    EventTimeEnum::Every { duration, .. } => {
                        last_fired.get(&event.name).map_or(true, |&last| current_time - last >= *duration)
                    },
                };

                if should_fire {
                    let strategy_event = StrategyEvent::TimedEvent(event.name.clone());
                    match self.strategy_event_sender.send(strategy_event).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Timed Event Handler: Failed to send event: {}", e)
                    }
                    last_fired.insert(event.name.clone(), current_time);

                    if let EventTimeEnum::DateTime { .. } = event.time {
                        events_to_remove.push(event.name.clone());
                    }
                    if let EventTimeEnum::Every { duration, ref mut next_time, .. } = event.time {
                        *next_time = current_time + duration;
                    }
                }
            }
        }
        schedule.retain(|e| !events_to_remove.contains(&e.name));
    }
}