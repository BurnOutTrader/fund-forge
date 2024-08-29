pub mod products;
pub mod accounts;
pub mod enums;
pub mod orders;
pub mod data_server_messaging;
pub mod subscriptions;
pub mod indicators;
pub mod base_data;
pub mod time_slices;
pub mod rolling_window;
pub mod subscription_handler;
pub mod strategy_events;

pub type OwnerId = String;
pub type TimeString = String;

pub type Price = f64;