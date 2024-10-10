use chrono::Utc;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::{fs, io};
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ff_rithmic_api::systems::RithmicSystem;
use structopt::StructOpt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::{join, task};
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor};
use tokio_rustls::server::TlsStream;
use ff_standard_lib::helpers::get_data_folder;
use ff_standard_lib::messages::data_server_messaging::{DataServerRequest, DataServerResponse};
use ff_standard_lib::server_features::rithmic_api::api_client::{RithmicClient, RITHMIC_CLIENTS};
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use crate::request_handlers::{manage_async_requests, stream_handler};
pub mod request_handlers;

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

async fn init_apis(options: ServerLaunchOptions) {
    let options = options.clone();
    tokio::task::spawn(async move {
        if !options.disable_rithmic_server {
            let mut toml_files = Vec::new();
            let dir = PathBuf::from(get_data_folder())
                .join("rithmic_credentials")
                .to_string_lossy()
                .into_owned();

            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();

                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        toml_files.push(file_name.to_string());
                    }
                }
            }

            for file in toml_files {
                if let Some(system) = RithmicSystem::from_file_string(file.as_str()) {
                    //todo add a bool option to credentials for is_data_feed
                    let client = RithmicClient::new(system, false).await.unwrap();
                    let client = Arc::new(client);
                    match RithmicClient::run_start_up(client.clone(), true, true).await {
                        Ok(_) => {}
                        Err(e) => eprintln!("Fail to run rithmic client for: {}, reason: {}", system, e)
                    }
                    RITHMIC_CLIENTS.insert(system, client);
                } else {
                    eprintln!("Error parsing rithmic system from: {}", file);
                }
            }
        }
    });
}

async fn logout_apis(options: &ServerLaunchOptions) {
    if !options.disable_rithmic_server {
        let mut toml_files = Vec::new();
        let dir = PathBuf::from(get_data_folder())
            .join("rithmic_credentials")
            .to_string_lossy()
            .into_owned();

        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    toml_files.push(file_name.to_string());
                }
            }
        }

        for file in toml_files {
            if let Some(system) = RithmicSystem::from_file_string(file.as_str()) {
                if let Some(client) = RITHMIC_CLIENTS.get(&system) {
                    client.shutdown().await;
                }
            }
        }
    }
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

    init_apis(options.clone()).await;

    let address = SocketAddr::new(options.listener_address, options.port);
    let async_server_handle = async_server(config.clone().into(), address, options.data_folder.clone()).await;
    let address = SocketAddr::new(options.stream_address.clone(), options.stream_port.clone());
    let stream_server_handle = stream_server(config.into(), address).await;

    let (stream_result, async_result) = join!(stream_server_handle, async_server_handle);

    if let Err(e) = async_result {
        eprintln!("Asynchronous server failed: {:?}", e);
    }

    if let Err(e) = stream_result {
        eprintln!("Stream server failed: {:?}", e);
    }

    logout_apis(&options).await;
    Ok(())
}

async fn get_ip_addresses(stream: &TlsStream<TcpStream>) -> SocketAddr {
    let tcp_stream = stream.get_ref();
    tcp_stream.0.peer_addr().unwrap()
}

pub(crate) async fn async_server(config: ServerConfig, addr: SocketAddr, _data_folder: PathBuf) -> JoinHandle<()> {
    const LENGTH: usize = 8;
    task::spawn(async move {
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = match TcpListener::bind(&addr).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Server: Failed to listen on {}: {}", addr, e);
                return;
            }
        };
        println!("Listening on: {}", addr);

        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok((stream, peer_addr)) => (stream, peer_addr),
                Err(e) => {
                    eprintln!("Server: Failed to accept TLS connection: {:?}", e);
                    continue;
                }
            };
            println!("Server: {}, peer_addr: {:?}", Utc::now(), peer_addr);
            let acceptor = acceptor.clone();

            let mut tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Server: Failed to accept TLS connection: {:?}", e);
                    return;
                }
            };

            let mut length_bytes = [0u8; LENGTH];
            let mut mode = StrategyMode::Backtest;
            'registration_loop: while let Ok(_) = tls_stream.read_exact(&mut length_bytes).await {
                // Parse the length from the header
                let msg_length = u64::from_be_bytes(length_bytes) as usize;
                let mut message_body = vec![0u8; msg_length];

                // Read the message body based on the length
                match tls_stream.read_exact(&mut message_body).await {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Server: Error reading message body: {}", e);
                        continue;
                    }
                }

                // Parse the request from the message body
                let request = match DataServerRequest::from_bytes(&message_body) {
                    Ok(req) => req,
                    Err(e) => {
                        eprintln!("Server: Failed to parse request: {:?}", e);
                        continue;
                    }
                };
                println!("{:?}", request);
                // Handle the request and generate a response
                match request {
                    DataServerRequest::Register(registered_mode) => {
                        mode = registered_mode;
                        break 'registration_loop
                    },
                    _ => eprintln!("Server: Strategy Did not register a Strategy mode")
                }
            }
            println!("Server: TLS connection established with {:?}", peer_addr);
            let stream_name = get_ip_addresses(&tls_stream).await.port();

            // If we are using live stream send the stream response so that the strategy can
            if mode == StrategyMode::Live || mode == StrategyMode::LivePaperTrading {
                let response = DataServerResponse::RegistrationResponse(stream_name.clone());
                // Convert the response to bytes
                let bytes = response.to_bytes();

                // Prepare the message with a 4-byte length header in big-endian format
                let length = (bytes.len() as u64).to_be_bytes();
                let mut prefixed_msg = Vec::new();
                prefixed_msg.extend_from_slice(&length);
                prefixed_msg.extend_from_slice(&bytes);

                // Write the response to the stream
                match tls_stream.write_all(&prefixed_msg).await {
                    Err(_e) => {
                        // Handle the error (log it or take some other action)
                    }
                    Ok(_) => {
                        // Successfully wrote the message
                    }
                }
            }

            manage_async_requests(
                mode,
                tls_stream,
                stream_name
            )
            .await;
        }
    })
}

pub(crate) async fn stream_server(config: ServerConfig, addr: SocketAddr) -> JoinHandle<()> {
    const LENGTH: usize = 8;
    task::spawn(async move {
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = match TcpListener::bind(&addr).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Stream: Failed to listen on {}: {}", addr, e);
                return;
            }
        };
        println!("Stream: Listening on: {}", addr);

        'main_loop: loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok((stream, peer_addr)) => (stream, peer_addr),
                Err(e) => {
                    eprintln!("Stream: Failed to accept TLS connection: {:?}", e);
                    continue;
                }
            };
            println!("Stream: {}, peer_addr: {:?}", Utc::now(), peer_addr);
            let acceptor = acceptor.clone();

            let mut tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Stream: Failed to accept TLS connection: {:?}", e);
                    return;
                }
            };

            let mut length_bytes = [0u8; LENGTH];
            while let Ok(_) = tls_stream.read_exact(&mut length_bytes).await {
                // Parse the length from the header
                let msg_length = u64::from_be_bytes(length_bytes) as usize;
                let mut message_body = vec![0u8; msg_length];

                // Read the message body based on the length
                match tls_stream.read_exact(&mut message_body).await {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Stream: Error reading message body: {}", e);
                        continue;
                    }
                }

                // Parse the request from the message body
                let request = match DataServerRequest::from_bytes(&message_body) {
                    Ok(req) => req,
                    Err(e) => {
                        eprintln!("Stream: Failed to parse request: {:?}", e);
                        continue;
                    }
                };
                println!("{:?}", request);

                // Handle the request and generate a response
                 match request {
                    DataServerRequest::RegisterStreamer(port) => {
                        stream_handler(tls_stream, port).await;
                        continue 'main_loop
                    },
                    _ => eprintln!("Stream: Strategy Did not register a Strategy mode")
                }
            }
            println!("Stream: TLS connection established with {:?}", peer_addr);
        }
    })
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
