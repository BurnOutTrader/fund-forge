use rustls::ServerConfig;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use chrono::Utc;
use tokio_rustls::server::TlsStream;
use ff_standard_lib::messages::data_server_messaging::{DataServerRequest, DataServerResponse};
use ff_standard_lib::standardized_types::enums::StrategyMode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use crate::request_handlers::manage_async_requests;

pub(crate) async fn async_server(config: ServerConfig, addr: SocketAddr, _data_folder: PathBuf) {
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

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    handle_async_connection(tls_stream, peer_addr).await;
                }
                Err(e) => {
                    eprintln!("Server: Failed to accept TLS connection: {:?}", e);
                }
            }
        });
    }
}

async fn handle_async_connection(mut tls_stream: TlsStream<TcpStream>, peer_addr: SocketAddr) {
    const LENGTH: usize = 8;
    let mut length_bytes = [0u8; LENGTH];
    let mut mode = StrategyMode::Backtest;
    while let Ok(_) = tls_stream.read_exact(&mut length_bytes).await {
        // Parse the length from the header
        let msg_length = u64::from_be_bytes(length_bytes) as usize;
        let mut message_body = vec![0u8; msg_length];

        // Read the message body based on the length
        match tls_stream.read_exact(&mut message_body).await {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Server: Error reading message body: {}", e);
                return;
            }
        }

        // Parse the request from the message body
        let request = match DataServerRequest::from_bytes(&message_body) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Server: Failed to parse request: {:?}", e);
                return;
            }
        };
        println!("{:?}", request);
        // Handle the request and generate a response
        match request {
            DataServerRequest::Register(registered_mode) => {
                mode = registered_mode;
                break;
            },
            _ => eprintln!("Server: Strategy Did not register a Strategy mode")
        }
    }
    println!("Server: TLS connection established with {:?}", peer_addr);
    let stream_name = crate::get_ip_addresses(&tls_stream).await.port();

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
        if let Err(_e) = tls_stream.write_all(&prefixed_msg).await {
            return;
            // Handle the error (log it or take some other action)
        }
    }

    manage_async_requests(
        mode,
        tls_stream,
        stream_name
    ).await;
}