use std::fs::File;
use std::io;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task;
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor, TlsStream};
use ff_standard_lib::server_connections::ConnectionType;
use ff_standard_lib::servers::request_handlers::manage_sequential_requests;
use ff_standard_lib::servers::settings::client_settings::get_settings;
use ff_standard_lib::servers::communications_sync::{SecureExternalCommunicator, SynchronousCommunicator};

#[derive(Debug)]
pub enum StartUpMode {
    Toml,
    CmdLine,
}

#[derive(Debug, StructOpt)]
struct ServerLaunchOptions {
    /// Sets the data folder
    #[structopt(short = "f", long = "data_folder", parse(from_os_str), default_value = "/Users/kevmonaghan/RustroverProjects/fund-forge/ff_data_server/data")]
    pub data_folder: PathBuf,

    #[structopt(short = "l", long = "ssl_folder", parse(from_os_str), default_value = "/Users/kevmonaghan/RustroverProjects/fund-forge/resources/keys")]
    pub ssl_auth_folder: PathBuf,

    #[structopt(short = "s", long = "synchronous_address", default_value = "0.0.0.0:8080")]
    pub listener_address: String,

    #[structopt(short = "p", long = "async_address", default_value = "0.0.0.0:8081")]
    pub async_listener_address: String,

/*    #[structopt(short = "c", long = "connection_type"]
    pub start_up_mode: String,*/
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let options = ServerLaunchOptions::from_args();
    let settings = get_settings(&ConnectionType::Default).await.unwrap();

    let cert = Path::join(&options.ssl_auth_folder, "cert.pem");
    let key = Path::join(&options.ssl_auth_folder, "key.pem");

    let certs = load_certs(&cert).unwrap();
    let key = load_keys(&key).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No keys found")).unwrap();

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err)).unwrap();

    let sync_server_handle = synchronous_server(config.clone().into(), settings.address_synchronous).await;
    let async_server_handle = async_server(config.into(), settings.address).await;

    let sync_result = sync_server_handle.await;
    let async_result = async_server_handle.await;

    if let Err(e) = sync_result {
        eprintln!("Synchronous server failed: {:?}", e);
    }

    if let Err(e) = async_result {
        eprintln!("Asynchronous server failed: {:?}", e);
    }

    Ok(())
}

/*pub async fn toml_launch(connection_type: ConnectionType) -> JoinHandle<()> {
    task::spawn(async move {
        let settings = get_settings(&connection_type).await.unwrap();

        let cert = Path::join(&settings.ssl_auth_folder, "cert.pem");
        let key = Path::join(&settings.ssl_auth_folder, "key.pem");

        let certs = load_certs(&cert).unwrap();
        let key = load_keys(&key).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No keys found")).unwrap();

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err)).unwrap();

        let sync_server_handle = synchronous_server(config.clone().into(), settings.address_synchronous).await;
        let async_server_handle = async_server(config.into(), settings.address).await;

        let sync_result = sync_server_handle.await;
        let async_result = async_server_handle.await;

        if let Err(e) = sync_result {
            eprintln!("Synchronous server failed: {:?}", e);
        }

        if let Err(e) = async_result {
            eprintln!("Asynchronous server failed: {:?}", e);
        }
    })
}*/

pub(crate) async fn synchronous_server(config: ServerConfig, addr: SocketAddr) -> JoinHandle<()>  {
    task::spawn(async move {
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = match TcpListener::bind(&addr).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to listen on {}: {}", addr, e);
                return;
            }
        };
        println!("Listening on: {}", addr);

        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok((stream, peer_addr)) => (stream, peer_addr),
                Err(e) => {
                    eprintln!("Failed to accept TLS connection: {:?}", e);
                    continue;
                }
            };
            println!("peer_addr: {:?}", peer_addr);
            let acceptor = acceptor.clone();

            let tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Failed to accept TLS connection: {:?}", e);
                    continue;
                }
            };
            let communicator = SynchronousCommunicator::new(SynchronousCommunicator::TlsConnections(SecureExternalCommunicator::new(Arc::new(Mutex::new(TlsStream::from(tls_stream))))));
            manage_sequential_requests(Arc::new(communicator)).await;

            println!("TLS connection established with {:?}", peer_addr);
        }
    })
}

pub(crate) async fn async_server(config: ServerConfig, addr: SocketAddr) -> JoinHandle<()>  {
    task::spawn(async move {
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = match TcpListener::bind(&addr).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to listen on {}: {}", addr, e);
                return;
            }
        };
        println!("Listening on: {}", addr);

        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok((stream, peer_addr)) => (stream, peer_addr),
                Err(e) => {
                    eprintln!("Failed to accept TLS connection: {:?}", e);
                    continue;
                }
            };
            println!("peer_addr: {:?}", peer_addr);
            let acceptor = acceptor.clone();

            let tls_stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Failed to accept TLS connection: {:?}", e);
                    continue;
                }
            };

            println!("TLS connection established with {:?}", peer_addr);
        }
    })
}






pub(crate) fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    let certificates =  certs(&mut BufReader::new(File::open(path)?)).collect();
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



