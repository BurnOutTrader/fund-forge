pub mod gui_types;
pub mod helpers;
pub mod communicators;
pub mod standardized_types;
pub mod messages;
pub mod strategies;

#[cfg(feature = "server")]
pub mod server_features;

pub type StreamName = u16;
