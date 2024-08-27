use std::sync::mpsc::Sender;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use tokio::sync::RwLock;

pub enum EventTimeEnum {
    Weekday {
        day: Weekday,
    },
    HourOfDay {
        hour: u32,
    },
    HourOnWeekDay {
        day: Weekday,
        hour: u32,
    },
    DateTime{
        date_time: DateTime<Utc>,
    },
    TimeOfDay {
        hour: u32,
        minute: u32,
        second: u32,
    },
    Every {
        duration: Duration,
        next_time: DateTime<Utc>,
    }
}

impl EventTimeEnum {
    pub fn is_time(&mut self, current_time: DateTime<Utc>) -> bool {
        match self {
            EventTimeEnum::Weekday { day } => {
                if current_time.weekday() == *day {
                    return true
                }
            },
            EventTimeEnum::HourOfDay { hour } => {
                if current_time.hour() == *hour {
                    return true
                }
            },
            EventTimeEnum::HourOnWeekDay { day, hour } => {
                if current_time.weekday() == *day && current_time.hour() == *hour {
                    return true
                }
            },
            EventTimeEnum::DateTime { date_time } => {
                if current_time == *date_time {
                    return true
                }
            },
            EventTimeEnum::TimeOfDay { hour, minute, second } => {
                if current_time.hour() == *hour && current_time.minute() == *minute && current_time.second() == *second {
                    return true
                }
            },
            EventTimeEnum::Every { duration, mut next_time } => {
                if current_time >= next_time {
                    next_time = current_time + *duration;
                    return true
                }
            }
        }
        false
    }
}

pub struct TimedEvent {
    name: String,
    event_time: EventTimeEnum,
    sender: Sender<DateTime<Utc>>,
}

impl TimedEvent {
    pub fn new(name: String, event_time: EventTimeEnum, sender: Sender<DateTime<Utc>>) -> Self {
        TimedEvent {
            name,
            event_time,
            sender,
        }
    }
}

pub struct TimedEventHandler {
    pub(crate) schedule: RwLock<Vec<TimedEvent>>,
}

impl TimedEventHandler {
    pub fn new() -> Self {
        TimedEventHandler {
            schedule: Default::default(),
        }
    }

    pub async fn add_event(&self, scheduled_event: TimedEvent) {
        self.schedule.write().await.push(scheduled_event);
    }

    pub async fn remove_event(&self, name: String) {
        self.schedule.write().await.retain(|event| event.name != name);
    }

    pub async fn update_time(&self, current_time: DateTime<Utc>) {
        let mut schedule = self.schedule.write().await;
        if schedule.len() == 0 {
            return;
        }
        let mut events_to_remove = vec![];
        for event in schedule.iter_mut() {
            if event.event_time.is_time(current_time) {
                match event.sender.send(current_time) {
                    Ok(_) => {},
                    Err(_) => {}
                }
                if let EventTimeEnum::DateTime { .. } = event.event_time {
                    events_to_remove.push(event.name.clone());
                }
            }
        }
        schedule.retain(|e| !events_to_remove.contains(&e.name));
    }
}