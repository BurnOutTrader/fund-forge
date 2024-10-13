pub mod gui_types;
pub mod helpers;
pub mod communicators;
pub mod standardized_types;
pub mod messages;
pub mod strategies;

#[cfg(feature = "server")]
pub mod server_features;

/// The `stream_name` is just the u16 port number of the strategy which the server is connecting to,
/// it is used to link the streaming port to a async port, you just need to know it represents a single strategy instance.
/// This allows you to create logic per connecting strategy, so you can drop objects from memory when a strategy goes offline.
pub type StreamName = u16;
