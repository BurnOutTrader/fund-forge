use std::path::PathBuf;

pub mod converters;
pub mod decimal_calculators;

/// This just needs to point to your fund-forge/ff_data_server/data folder, we need this so that when we eventually run the server on a cloud or collocated service, we can persist our data between instances.
/// I am aware this is not an optimal way of doing things but it will do for now
pub fn get_data_folder() -> PathBuf {
    PathBuf::from("./data")
}

/// this just points to your fund-forge/resources folder, where all SSL key and server configuration toml file is located.
/// I am aware this is not an optimal way of doing things but it will do for now
pub fn get_resources() -> PathBuf {
    PathBuf::from("./resources")
}

pub fn get_toml_file_path() -> PathBuf {
    let resources = get_resources();
    resources.join("server_settings.toml")
}
