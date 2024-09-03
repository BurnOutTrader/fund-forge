use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use std::sync::mpsc::Sender;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
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
    sender: Sender<TimedEvent>,
}

impl TimedEvent {
    pub fn new(name: String, event_time: EventTimeEnum, sender: Sender<TimedEvent>) -> Self {
        TimedEvent {
            name,
            time: event_time,
            sender,
        }
    }
}

pub struct TimedEventHandler {
    pub(crate) schedule: RwLock<Vec<TimedEvent>>,
    is_warmed_up: RwLock<bool>,
}

impl TimedEventHandler {
    pub fn new() -> Self {
        TimedEventHandler {
            schedule: Default::default(),
            is_warmed_up: RwLock::new(false),
        }
    }

    pub async fn set_warmup_complete(&self) {
        *self.is_warmed_up.write().await = true;
    }

    pub async fn add_event(&self, scheduled_event: TimedEvent) {
        self.schedule.write().await.push(scheduled_event);
    }

    pub async fn remove_event(&self, name: String) {
        self.schedule
            .write()
            .await
            .retain(|event| event.name != name);
    }

    pub async fn update_time(&self, current_time: DateTime<Utc>) {
        let mut schedule = self.schedule.write().await;
        if schedule.len() == 0 {
            return;
        }
        let mut events_to_remove = vec![];
        for event in schedule.iter_mut() {
            if event.time.event_time(current_time) {
                match event.sender.send(event.clone()) {
                    Ok(_) => {}
                    Err(_) => {}
                }
                if let EventTimeEnum::DateTime { .. } = event.time {
                    events_to_remove.push(event.name.clone());
                }
                if let EventTimeEnum::Every { duration, mut next_time,..} = event.time
                {
                    next_time = current_time + duration;
                }
            }
        }
        schedule.retain(|e| !events_to_remove.contains(&e.name));
    }
}
