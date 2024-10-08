use crate::server_features::api_modes::Mode;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::messages::data_server_messaging::ApiKey;

pub struct ApiSettings {
    pub connection_type: ConnectionType,
    pub api_key: ApiKey,
    pub mode: Mode,
    pub max_concurrent_downloads: i32,
    pub activate: bool,
}
