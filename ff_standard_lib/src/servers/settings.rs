pub mod client_settings {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::path::{PathBuf};
    use std::str::FromStr;
    use std::sync::Arc;
    use serde_derive::{Deserialize, Serialize};
    use tokio::sync::Mutex;
    use crate::server_connections::{ConnectionType};
    use crate::standardized_types::data_server_messaging::{FundForgeError};
    use once_cell::sync::Lazy;
    use crate::helpers::{get_resources, get_toml_file_path};

    static CONNECTION_SETTINGS: Lazy<Result<Arc<Mutex<HashMap<ConnectionType, ConnectionSettings>>>, FundForgeError>> = Lazy::new(|| {
        initialise_settings()
    });

    #[derive(Debug, Serialize, Deserialize)]
    struct SettingsMap {
        settings: HashMap<String, ConnectionSettings>
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
    pub fn initialise_settings() -> Result<Arc<Mutex<HashMap<ConnectionType, ConnectionSettings>>>, FundForgeError> {
        let toml_file_path = get_toml_file_path();

        if !toml_file_path.exists() {
            let default_settings = ConnectionSettings::default();
            let mut map: HashMap<ConnectionType, ConnectionSettings> = HashMap::new();
            map.insert(ConnectionType::Default, default_settings);
            // save map as server_settings.toml in "../resources" folder
            let mut map_set: HashMap<String, ConnectionSettings> = HashMap::new();
            for (connection_type, settings) in map.iter() {
                map_set.insert(connection_type.to_string(), settings.clone());
            }
            let settings_map = SettingsMap {
                settings: map_set
            };
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
            map.insert(ConnectionType::from_str(c_type_string).unwrap(), setting.clone());
        }

        let map = Arc::new(Mutex::new(map));
        Ok(map)
    }


    /// Retrieves a shared reference to the global client connection settings map.
    ///
    /// This function accesses a lazily-initialized, global instance of the client connection settings,
    /// which is stored in a `HashMap` keyed by `ConnectionType`. The settings map is wrapped in an `Arc<Mutex<>>`
    /// to allow safe, concurrent access across multiple threads and async tasks. If the settings have not been
    /// successfully initialized prior to this call (e.g., due to an error loading from a configuration file),
    /// this function will panic, indicating the nature of the initialization error.
    ///
    /// # Returns
    /// An `Arc<Mutex<HashMap<ConnectionType, ClientConnectionSettings>>>`: A thread-safe, reference-counted
    /// wrapper around the global settings map. This allows the caller to access and potentially modify the
    /// settings in a controlled manner.
    ///
    /// # Panics
    /// This function will panic if the global settings have not been successfully initialized, with the panic
    /// message providing details about the initialization error.
    pub fn get_settings_map() -> Arc<Mutex<HashMap<ConnectionType, ConnectionSettings>>> {
        match &*CONNECTION_SETTINGS {
            Ok(settings_arc) => {
                settings_arc.clone()
            },
            Err(e) => panic!("{:?}", e)
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug)]
    pub struct ConnectionSettings {
        /// The path to the folder containing the SSL certificate and private key files.
        pub ssl_auth_folder: PathBuf,
        /// The name of the server as specified when creating the ssl certificate and private key files
        pub server_name: String,
        /// the listener for async streaming type communications
        pub address: SocketAddr,
        ///the listener for synchrnous rest type communications
        pub address_synchronous: SocketAddr,
    }

    impl Default for ConnectionSettings {
        fn default() -> Self {
            ConnectionSettings {
                ssl_auth_folder: get_resources().join("keys"),
                server_name: String::from("fundforge"),
                address:  SocketAddr::from_str("127.0.0.1:8080").unwrap(),
                address_synchronous: SocketAddr::from_str("127.0.0.1:8081").unwrap()
            }
        }
    }

    /// Retrieves the client connection settings for a specified connection type.
    ///
    /// This function looks up the `CONNECTION_SETTINGS` map for the given `connection_type`.
    /// If the settings for the specified type are found, they are returned directly. Otherwise,
    /// the function attempts to initialize the settings by reading from the default TOML configuration file,
    /// updates the map with these settings, and then returns them.
    ///
    /// # Parameters
    /// - `connection_type`: A reference to the `ConnectionType` for which the settings are being requested.
    ///
    /// # Returns
    /// - `Ok(ClientConnectionSettings)`: The connection settings for the specified type if found or successfully loaded.
    /// - `Err(FundForgeError)`: An error of type `FundForgeError` if the settings cannot be loaded.
    ///
    /// # Errors
    /// This function can return an error if:
    /// - The default TOML configuration file cannot be read or parsed.
    /// - The specified `connection_type` does not exist in the loaded settings.
    ///
    /// # Examples
    /// ```rust
    ///     use std::fs;
    ///     use std::net::SocketAddr;
    ///     use std::path::{Path};
    ///     use ff_standard_lib::server_connections::ConnectionType;
    ///     use ff_standard_lib::servers::settings::client_settings::get_settings;
    ///
    ///     /// Tests loading of settings from a TOML file and validates the certificate file path.
    ///     #[tokio::test]
    ///     async fn test_load_and_validate_settings() {
    ///        match get_settings(&ConnectionType::Default).await {
    ///             Ok(settings) => {
    ///                 // Validate the ca_file exists
    ///                 let ca_file = Path::join(&settings.ssl_auth_folder, "rootCA.crt");
    ///                 println!("{:?}", ca_file);
    ///                 assert!(fs::metadata(&ca_file).is_ok(), "CA file does not exist");
    ///
    ///                 // Optionally, add more checks here for server_name, address, etc.
    ///                 assert_eq!(settings.server_name, "fundforge");
    ///                 assert_eq!(settings.address, "127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    ///             }
    ///             Err(e) => panic!("Failed to get settings: {:?}", e),
    ///         }
    ///     }
    /// ```
    pub async fn get_settings(connection_type:  &ConnectionType) -> Result<ConnectionSettings, FundForgeError> {
        let map = get_settings_map().clone();
        let map = map.lock().await;
        if map.contains_key(&connection_type) {
            Ok(map.get(&connection_type).unwrap().clone())
        }
        else {
            Err(FundForgeError::ClientSideErrorDebug(String::from("Connection type not found in settings.")))
        }
    }
}
