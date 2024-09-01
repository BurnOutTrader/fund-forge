use std::path::PathBuf;

pub mod converters;
pub mod decimal_calculators;

pub fn get_data_folder() -> PathBuf {
    PathBuf::from("/Users/kevmonaghan/RustroverProjects/fund-forge/ff_data_server/data")
}

pub fn get_resources() -> PathBuf {
    PathBuf::from("/Users/kevmonaghan/RustroverProjects/fund-forge/resources")
}

pub fn get_toml_file_path() -> PathBuf {
    let resources = get_resources();
    resources.join("server_settings.toml")
}
