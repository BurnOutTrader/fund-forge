use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::net::{SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use async_std::task::sleep;
use once_cell::sync::Lazy;
use structopt::StructOpt;
use tokio::net::TcpStream;
use tokio::{signal, task};
use tokio::sync::{broadcast, OnceCell};
use tokio_rustls::server::TlsStream;
use ff_standard_lib::database::hybrid_storage::{HybridStorage};
use ff_standard_lib::server_launch_options::ServerLaunchOptions;
use crate::data_bento_api::api_client::{data_bento_init};
use crate::oanda_api::api_client::{oanda_init};
use crate::rithmic_api::api_client::{RithmicBrokerageClient, RITHMIC_CLIENTS};
use crate::update_functions::run_update_schedule;

pub mod request_handlers;
mod stream_listener;
mod async_listener;
pub mod rithmic_api;
pub mod data_bento_api;
pub mod rate_limiter;
pub mod server_side_brokerage;
pub mod server_side_datavendor;
pub mod bitget_api;
pub mod stream_tasks;
pub mod oanda_api;
pub mod server_features;
pub mod fred;
pub mod update_functions;
use crate::update_functions::DATA_STORAGE;

async fn logout_apis() {
    println!("Logging Out Apis Function Started");
    if !RITHMIC_CLIENTS.is_empty() {
        for api_client in RITHMIC_CLIENTS.iter() {
            api_client.shutdown().await;
        }
    }
    println!("Logging Out Apis Function Ended");
}

static SHUTDOWN_CHANNEL: Lazy<broadcast::Sender<()>> = Lazy::new(|| {
    let (sender, _) = broadcast::channel(20);
    sender
});

// Function to get_requests the sender
fn get_shutdown_sender() -> &'static broadcast::Sender<()> {
    &SHUTDOWN_CHANNEL
}

// Function to get_requests a new receiver
pub fn subscribe_server_shutdown() -> broadcast::Receiver<()> {
    SHUTDOWN_CHANNEL.subscribe()
}

static DATA_FOLDER: OnceCell<PathBuf> = OnceCell::const_new();
pub fn get_data_folder() -> &'static PathBuf {
    DATA_FOLDER.get().expect("DATA_FOLDER has not been initialized.")
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let options = ServerLaunchOptions::from_args();
    let _ = DATA_FOLDER.set(options.data_folder.clone());
    println!("Data Folder: {:?}", get_data_folder());
    let _ = DATA_STORAGE.set(Arc::new(HybridStorage::new(Duration::from_secs(450), options.clone(), options.max_downloads, options.update_seconds)));

    // Start the background task for cache management
    HybridStorage::start_cache_management(DATA_STORAGE.get().unwrap().clone());

    let cert = Path::join(&options.ssl_auth_folder, "cert.pem");
    let key = Path::join(&options.ssl_auth_folder, "key.pem");

    let certs = load_certs(&cert)?;
    let key = load_keys(&key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No keys found"))?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    RithmicBrokerageClient::init_rithmic_apis(options.clone()).await;
    oanda_init(options.clone()).await;
    match data_bento_init(options.clone()).await {
        Ok(_) =>{
            eprintln!("Data Bento Initialized");
        /*    let client = get_data_bento_client().unwrap();
            client.symbols_response(StrategyMode::Backtest, 1, MarketType::Futures(FuturesExchange::CME), Some(Utc::now()), 1).await;*/
        }
        Err(_) => {},
    }


    run_servers(config, options.clone());

    sleep(Duration::from_secs(5)).await;

    run_update_schedule(DATA_STORAGE.get().unwrap().clone());

    // Wait for Ctrl+C
    signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
    println!("Ctrl+C received, logging out APIs...");
    match get_shutdown_sender().send(()) {
        Ok(_) => eprintln!("Shutdown Signal Sent"),
        Err(e) =>  eprintln!("Shutdown Signal Failed: {}", e),
    }

    // Perform logout
    logout_apis().await;


    println!("Shutdown complete");
    Ok(())
}

#[allow(dead_code)]
async fn get_ip_addresses(stream: &TlsStream<TcpStream>) -> SocketAddr {
    let tcp_stream = stream.get_ref();
    tcp_stream.0.peer_addr().unwrap()
}

fn run_servers(
    config: rustls::ServerConfig,
    options: ServerLaunchOptions,
) {
    let config_clone = config.clone();
    let options_clone = options.clone();

    let _ = task::spawn(async move  {
        async_listener::async_server(
            config_clone,
            SocketAddr::new(options_clone.listener_address, options_clone.port),
        ).await
    });

    let _ = task::spawn(async move  {
         stream_listener::stream_server(
            config,
            SocketAddr::new(options.stream_address, options.stream_port),
        ).await
    });
}

pub(crate) fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    let certificates = certs(&mut BufReader::new(File::open(path)?)).collect();
    //println!("certs: {:?}", certs(&mut BufReader::new(File::open(path)?)).collect::<Vec<_>>());
    certificates
}

pub(crate) fn load_keys(path: &Path) -> Option<PrivateKeyDer<'static>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return None,
    };
    private_key(&mut BufReader::new(file)).unwrap()
}
