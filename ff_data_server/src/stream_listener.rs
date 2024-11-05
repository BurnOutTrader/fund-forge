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
use ff_standard_lib::StreamName;
use crate::{subscribe_server_shutdown};
use crate::server_side_datavendor::{data_feed_subscribe, data_feed_unsubscribe};
use crate::stream_tasks::initialize_streamer;

#[allow(unused_assignments)]
pub(crate) async fn stream_server(config: ServerConfig, addr: SocketAddr) {
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = match TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Stream: Failed to listen on {}: {}", addr, e);
            return;
        }
    };
    let mut listener = Some(listener);
    let mut shutdown_receiver = subscribe_server_shutdown();
    println!("Stream: Listening on: {}", addr);

    loop {
        tokio::select! {
            // Accept new connection
            Ok((stream, peer_addr)) = listener.as_ref().unwrap().accept() => {
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

            // Listen for shutdown signal
            _ = shutdown_receiver.recv() => {
                println!("Stream: Shutdown signal received, stopping server.");
                 listener = None;
                break;
            }
        }
    }
    println!("Stream: Server stopped.");
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
        StreamRequest::Subscribe(subscription) => {
            //it is not when we subscribe that we need to update data, only when we request historical data
            data_feed_subscribe(stream_name, subscription).await
        }
        StreamRequest::Unsubscribe(sub) => {
            data_feed_unsubscribe(sub.symbol.data_vendor.clone(), stream_name, sub).await
        }
    }
}