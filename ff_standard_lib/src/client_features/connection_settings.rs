pub mod client_settings {
    use crate::helpers::{get_resources, get_toml_file_path};
    use crate::client_features::connection_types::ConnectionType;
    use crate::messages::data_server_messaging::FundForgeError;
    use serde_derive::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[derive(Debug, Serialize, Deserialize)]
    struct SettingsMap {
        settings: HashMap<String, ConnectionSettings>,
    }

    /// Initializes the client connection settings from a TOML file.
    ///
    /// This function attempts to load the client connection settings from a specified TOML file.
    /// If the file does not exist, it creates a default settings file with predefined values.
    /// The settings are then stored in a `HashMap` keyed by `ConnectionType`, which is wrapped
    /// in an `Arc<Mutex<>>` for thread-safe shared access across async tasks.
    ///
    /// # Parameters
    /// - `toml_file_path`: An `Option` containing a reference to a `Path` that specifies the location
    ///   of the TOML file to load the settings from. If `None`, a default path is used.
    ///
    /// # Returns
    /// - `Result<Arc<Mutex<HashMap<ConnectionType, ClientConnectionSettings>>>, FundForgeError>`:
    ///   On success, returns an `Arc<Mutex<>>` wrapping a `HashMap` of the settings keyed by `ConnectionType`.
    ///   On failure, returns a `FundForgeError` indicating what went wrong (e.g., file not found, parse error).
    ///
    /// # Errors
    /// This function can return an error in the following cases:
    /// - If there is an error reading from or writing to the TOML file.
    /// - If parsing the TOML content into the expected structure fails.
    pub fn initialise_settings(
    ) -> Result<HashMap<ConnectionType, ConnectionSettings>, FundForgeError> {
        let toml_file_path = get_toml_file_path();

        if !toml_file_path.exists() {
            let default_settings = ConnectionSettings::default();
            let mut map: HashMap<ConnectionType, ConnectionSettings> = HashMap::new();
            map.insert(ConnectionType::Default, default_settings);

            let dafault_registry_settings = ConnectionSettings {
                ssl_auth_folder: get_resources().join("keys"),
                server_name: String::from("fundforge"),
                address: SocketAddr::from_str("127.0.0.1:8082").unwrap(),
            };
            map.insert(ConnectionType::StrategyRegistry, dafault_registry_settings);

            // save map as server_settings.toml in "../resources" folder
            let mut map_set: HashMap<String, ConnectionSettings> = HashMap::new();
            for (connection_type, settings) in map.iter() {
                map_set.insert(connection_type.to_string(), settings.clone());
            }
            let settings_map = SettingsMap { settings: map_set };
            let toml_content = match toml::to_string(&settings_map) {
                Ok(content) => content,
                Err(e) => return Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
            };
            std::fs::write(toml_file_path.clone(), toml_content).unwrap();
        }

        let toml_content = match std::fs::read_to_string(toml_file_path) {
            Ok(content) => content,
            Err(e) => return Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        };

        let settings: SettingsMap = toml::from_str(&toml_content)
            .map_err(|e| FundForgeError::ClientSideErrorDebug(e.to_string()))?;

        let mut map: HashMap<ConnectionType, ConnectionSettings> = HashMap::new();
        for (c_type_string, setting) in &settings.settings {
            map.insert(
                ConnectionType::from_str(c_type_string).unwrap(),
                setting.clone(),
            );
        }

        Ok(map)
    }

    #[derive(Clone, Serialize, Deserialize, Debug)]
    pub struct ConnectionSettings {
        /// The path to the folder containing the SSL certificate and private key files.
        pub ssl_auth_folder: PathBuf,
        /// The name of the server as specified when creating the ssl certificate and private key files
        pub server_name: String,
        /// the listener for async streaming type communications
        pub address: SocketAddr,
    }

    impl Default for ConnectionSettings {
        fn default() -> Self {
            ConnectionSettings {
                ssl_auth_folder: get_resources().join("keys"),
                server_name: String::from("fundforge"),
                address: SocketAddr::from_str("127.0.0.1:8081").unwrap(),
            }
        }
    }
}
