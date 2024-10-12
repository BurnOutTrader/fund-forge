use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ff_rithmic_api::rithmic_proto_objects::rti::request_login::SysInfraType;
use ff_rithmic_api::rithmic_proto_objects::rti::RequestAccountRmsInfo;
use ff_rithmic_api::systems::RithmicSystem;
use futures::future::join_all;
use structopt::StructOpt;
use tokio::net::TcpStream;
use tokio::{signal, task};
use tokio_rustls::server::TlsStream;
use ff_standard_lib::server_features::rithmic_api::api_client::{RithmicClient, RITHMIC_CLIENTS};
pub mod request_handlers;
mod stream_listener;
mod async_listener;

#[derive(Debug, StructOpt, Clone)]
struct ServerLaunchOptions {
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
        short = "o",
        long = "stream_port",
        default_value = "8082"
    )]
    pub stream_port: u16,

    #[structopt(
        short = "r",
        long = "rithmic",
    )]
    pub disable_rithmic_server: bool,
}

#[allow(dead_code)]
async fn init_rithmic_apis(options: ServerLaunchOptions) {
    let options = options.clone();
    if options.disable_rithmic_server {
        return
    }
    let toml_files = RithmicClient::get_rithmic_tomls();
    if toml_files.is_empty() {
        return;
    }
    let init_tasks = toml_files.into_iter().filter_map(|file| {
        RithmicSystem::from_file_string(file.as_str()).map(|system| {
            task::spawn(async move {
                match RithmicClient::new(system).await {
                    Ok(client) => {
                        let client = Arc::new(client);
                        match RithmicClient::run_start_up(client.clone(), true, true).await {
                            Ok(_) => {
                                RITHMIC_CLIENTS.insert(system, client);
                                println!("Rithmic client initialized for: {}", system);
                            }
                            Err(e) => {
                                eprintln!("Failed to run rithmic client for: {}, reason: {}", system, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to create rithmic client for: {}, reason: {}", system, e);
                    }
                }
            })
        })
    }).collect::<Vec<_>>();

    // Wait for all initialization tasks to complete
    join_all(init_tasks).await;

    // Create a vector to hold all RMS request tasks
    let mut rms_tasks = Vec::new();

    for api in RITHMIC_CLIENTS.iter() {
        let api = api.value().clone();
        let rms_req = RequestAccountRmsInfo {
            template_id: 304,
            user_msg: vec![],
            fcm_id: api.credentials.fcm_id.clone(),
            ib_id: api.credentials.ib_id.clone(),
            user_type: api.credentials.user_type.clone(),
        };

        // Spawn a new task for each RMS request
        let task = task::spawn(async move {
            match api.client.send_message(SysInfraType::OrderPlant, rms_req).await {
                Ok(_) => println!("RMS request sent successfully for client: {}", api.system),
                Err(e) => eprintln!("Failed to send RMS request for client: {}, error: {}", api.system, e),
            }
        });

        rms_tasks.push(task);
    }

    // Wait for all RMS request tasks to complete
    join_all(rms_tasks).await;
}

async fn logout_apis() {
    println!("Logging Out Apis Function Started");
    if !RITHMIC_CLIENTS.is_empty() {
        for api_client in RITHMIC_CLIENTS.iter() {
            api_client.shutdown().await;
        }
    }
    println!("Logging Out Apis Function Ended");
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let options = ServerLaunchOptions::from_args();
    let cert = Path::join(&options.ssl_auth_folder, "cert.pem");
    let key = Path::join(&options.ssl_auth_folder, "key.pem");

    let certs = load_certs(&cert)?;
    let key = load_keys(&key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No keys found"))?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;


    //let init_rithmic_handle = tokio::spawn(init_rithmic_apis(options.clone()));
    let (_, _) = run_servers(config, options.clone());

    // Wait for initialization to complete
    //init_rithmic_handle.await.expect("Failed to initialize APIs");

    // Wait for Ctrl+C
    signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
    println!("Ctrl+C received, logging out APIs...");

    shutdown_stream_tasks();
    TEST_CLIENT.shutdown();

    // Perform logout
    logout_apis().await;

    println!("Shutdown complete");
    Ok(())
}

async fn get_ip_addresses(stream: &TlsStream<TcpStream>) -> SocketAddr {
    let tcp_stream = stream.get_ref();
    tcp_stream.0.peer_addr().unwrap()
}
use std::thread;
use tokio::runtime::Runtime;
use ff_standard_lib::server_features::stream_tasks::shutdown_stream_tasks;
use ff_standard_lib::server_features::test_api::api_client::TEST_CLIENT;

fn run_servers(
    config: rustls::ServerConfig,
    options: ServerLaunchOptions,
) -> (thread::JoinHandle<()>, thread::JoinHandle<()>) {
    let config_clone = config.clone();
    let options_clone = options.clone();

    let async_handle = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async_listener::async_server(
            config_clone,
            SocketAddr::new(options_clone.listener_address, options_clone.port),
            options_clone.data_folder,
        ));
    });

    let stream_handle = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(stream_listener::stream_server(
            config,
            SocketAddr::new(options.stream_address, options.stream_port),
        ));
    });

    (async_handle, stream_handle)
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
