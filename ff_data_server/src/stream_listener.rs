use rustls::ServerConfig;
use std::net::SocketAddr;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use chrono::Utc;
use tokio_rustls::server::TlsStream;
use ff_standard_lib::messages::data_server_messaging::{DataServerRequest, DataServerResponse, StreamRequest};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ff_standard_lib::server_features::StreamName;
use tokio::sync::mpsc;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use tokio::time::interval;
use tokio::sync::mpsc::Sender;
use ff_standard_lib::server_features::rithmic_api::api_client::RITHMIC_CLIENTS;
use ff_standard_lib::server_features::server_side_datavendor::VendorApiResponse;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use crate::request_handlers::{STREAM_CALLBACK_SENDERS};

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
                stream_handler(tls_stream, port, Duration::new(secs, subsec)).await;
                println!("Streamer Registered");
                return;
            },
            _ => eprintln!("Stream: Strategy Did not register a Strategy mode")
        }
    }
    println!("Stream: TLS connection established with {:?}", peer_addr);
}

pub async fn stream_handler(stream: TlsStream<TcpStream>, stream_name: StreamName, buffer: Duration) {
    let (stream_sender, stream_receiver) = mpsc::channel(1000);
    STREAM_CALLBACK_SENDERS.insert(stream_name.clone(), stream_sender.clone());
    let mut receiver = stream_receiver;
    let mut stream = stream;
    tokio::spawn(async move {
        let mut time_slice = TimeSlice::new();
        let mut interval = interval(buffer);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let bytes = time_slice.to_bytes();
                    // Prepare the message with a 4-byte length header in big-endian format
                    let length: [u8; LENGTH] = (bytes.len() as u32).to_be_bytes();
                    let mut prefixed_msg = Vec::new();
                    prefixed_msg.extend_from_slice(&length);
                    prefixed_msg.extend_from_slice(&bytes);
                    // Write the response to the stream
                    if let Err(e) = stream.write_all(&prefixed_msg).await {
                        eprintln!("Error writing to stream: {}", e);
                        break;
                    }
                    time_slice = TimeSlice::new();
                }
                Some(response) = receiver.recv() => {
                    time_slice.add(response);
                }
            }
        }
        STREAM_CALLBACK_SENDERS.remove(&stream_name);
    });
}

pub async fn shutdown_stream(stream_name: StreamName) {
    STREAM_CALLBACK_SENDERS.remove(&stream_name);
    for api in RITHMIC_CLIENTS.iter() {
        api.value().logout_command_vendors(stream_name).await;
    }
}

pub async fn stream_response(stream_name: StreamName, request: StreamRequest, sender: Sender<BaseDataEnum>) -> DataServerResponse {
    match request {
        StreamRequest::AccountUpdates(_, _) => {
            panic!()
        }
        StreamRequest::Subscribe(subscription) => {
            subscription.symbol.data_vendor.data_feed_subscribe(stream_name, subscription.clone(), sender).await
        }
        StreamRequest::Unsubscribe(_) => {
            panic!()
        }
    }
}