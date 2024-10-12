use rustls::ServerConfig;
use std::net::SocketAddr;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use chrono::Utc;
use tokio_rustls::server::TlsStream;
use ff_standard_lib::messages::data_server_messaging::{DataServerRequest, DataServerResponse, StreamRequest};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use ff_standard_lib::server_features::StreamName;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::server_features::stream_tasks::initialize_streamer;

pub(crate) async fn stream_server(config: ServerConfig, addr: SocketAddr) {
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = match TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Stream: Failed to listen on {}: {}", addr, e);
            return;
        }
    };
    println!("Stream: Listening on: {}", addr);

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok((stream, peer_addr)) => (stream, peer_addr),
            Err(e) => {
                eprintln!("Stream: Failed to accept TLS connection: {:?}", e);
                continue;
            }
        };
        println!("Stream: {}, peer_addr: {:?}", Utc::now(), peer_addr);
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    handle_stream_connection(tls_stream, peer_addr).await;
                }
                Err(e) => {
                    eprintln!("Stream: Failed to accept TLS connection: {:?}", e);
                }
            }
        });
    }
}

const LENGTH: usize = 4;
async fn handle_stream_connection(mut tls_stream: TlsStream<TcpStream>, peer_addr: SocketAddr) {
    let mut length_bytes = [0u8; LENGTH];
    while let Ok(_) = tls_stream.read_exact(&mut length_bytes).await {
        // Parse the length from the header
        let msg_length = u32::from_be_bytes(length_bytes) as usize;
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
            DataServerRequest::RegisterStreamer{port, secs, subsec } => {
                initialize_streamer(port, Duration::new(secs, subsec), tls_stream).await;
                println!("Streamer Registered");
                return;
            },
            _ => eprintln!("Stream: Strategy Did not register a Strategy mode")
        }
    }
    println!("Stream: TLS connection established with {:?}", peer_addr);
}


pub async fn stream_response(stream_name: StreamName, request: StreamRequest) -> DataServerResponse {
    match request {
        StreamRequest::AccountUpdates(_, _) => {
            panic!()
        }
        StreamRequest::Subscribe(subscription) => {
            subscription.symbol.data_vendor.data_feed_subscribe(stream_name, subscription.clone()).await
        }
        StreamRequest::Unsubscribe(_) => {
            panic!()
        }
    }
}