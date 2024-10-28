use chrono::{DateTime, Utc};

pub(crate) mod handle_tick_plant;
pub(crate) mod handle_order_plant;
pub(crate) mod handle_pnl_plant;
pub(crate) mod handle_history_plant;
pub(crate) mod handle_repo_plant;
pub(crate) mod reconnect;
pub(crate) mod handler_loop;

pub(crate) fn create_datetime(ssboe: i64, usecs: i64) -> DateTime<Utc> {
    let nanosecs = usecs * 1000; // Convert microseconds to nanoseconds
    DateTime::<Utc>::from_timestamp(ssboe, nanosecs as u32).unwrap()
}