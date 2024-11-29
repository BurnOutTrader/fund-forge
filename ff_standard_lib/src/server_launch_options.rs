use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
#[allow(dead_code)]
pub struct ServerLaunchOptions {
    /// Sets the data folder
    #[structopt(
        short = "f",
        long = "data_folder",
        parse(from_os_str),
        default_value = "./data"
    )]
    pub data_folder: PathBuf,

    #[structopt(
        short = "l",
        long = "ssl_folder",
        parse(from_os_str),
        default_value = "./resources/keys"
    )]
    pub ssl_auth_folder: PathBuf,

    #[structopt(
        short = "a",
        long = "address",
        default_value = "127.0.0.1"
    )]
    pub listener_address: IpAddr,

    #[structopt(
        short = "p",
        long = "port",
        default_value = "8081"
    )]
    pub port: u16,

    #[structopt(
        short = "s",
        long = "stream_address",
        default_value = "127.0.0.1"
    )]
    pub stream_address: IpAddr,

    #[structopt(
        short = "x",
        long = "stream_port",
        default_value = "8082"
    )]
    pub stream_port: u16,

    #[structopt(
        short = "r",
        long = "rithmic",
        default_value = "0"
    )]
    pub disable_rithmic_server: u64,

    #[structopt(
        short = "o",
        long = "oanda",
        default_value = "0"
    )]
    pub disable_oanda_server: u64,

    #[structopt(
        short = "b",
        long = "bitget",
        default_value = "0"
    )]
    pub disable_bitget_server: u64,

    /// Sets the maximum number of concurrent downloads
    #[structopt(
        short = "m",
        long = "downloads",
        default_value = "5"
    )]
    pub max_downloads: usize,

    /// Sets the update interval in seconds
    #[structopt(
        short = "u",
        long = "updates",
        default_value = "900"
    )]
    pub update_seconds: u64,
}
impl Default for ServerLaunchOptions {
    fn default() -> Self {
        ServerLaunchOptions {
            data_folder: PathBuf::from("./data"),
            ssl_auth_folder: PathBuf::from("./resources/keys"),
            listener_address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 8081,
            stream_address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            stream_port: 8082,
            disable_rithmic_server: 0,
            disable_oanda_server: 0,
            disable_bitget_server: 0,
            max_downloads: 20,
            update_seconds: 900,
        }
    }
}
