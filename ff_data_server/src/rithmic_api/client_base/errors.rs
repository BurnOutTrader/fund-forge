use thiserror::Error;
use std::io;
use tungstenite::Error as WsError;
use prost::DecodeError;

#[derive(Debug, Error)]
pub enum RithmicApiError {
    #[error("Server error: {0}")]
    ServerErrorDebug(String),

    #[error("Client error: {0}")]
    ClientErrorDebug(String),

    #[error("IO error occurred: {0}")]
    Io(#[from] io::Error),

    #[error("WebSocket error occurred: {0}")]
    WebSocket(#[from] WsError),

    #[error("Protobuf decode error: {0}")]
    ProtobufDecode(#[from] DecodeError),

    #[error("Disconnected error: {0}")]
    Disconnected(String),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Invalid server name: {0}")]
    InvalidServerName(String),

    #[error("Invalid config: {0}")]
    InvalidConfig(String)
}