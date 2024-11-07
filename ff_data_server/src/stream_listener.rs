use rustls::ServerConfig;
use std::net::SocketAddr;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use tokio::net::{TcpStream};
use tokio_rustls::server::TlsStream;
use ff_standard_lib::messages::data_server_messaging::{DataServerRequest, DataServerResponse, StreamRequest};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use ff_standard_lib::StreamName;
use crate::{subscribe_server_shutdown};
use crate::server_side_datavendor::{data_feed_subscribe, data_feed_unsubscribe};
use crate::stream_tasks::initialize_streamer;
use tokio::sync::Notify;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::async_listener::create_listener;

pub(crate) async fn stream_server(config: ServerConfig, addr: SocketAddr) {
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = match create_listener(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Stream: Failed to create listener on {}: {}", addr, e);
            return;
        }
    };

    println!("Stream: Listening on: {}", addr);

    let mut shutdown_receiver = subscribe_server_shutdown();
    let active_connections = Arc::new(AtomicUsize::new(0));
    let listener = Arc::new(listener);
    let shutdown_complete_tx = Arc::new(Notify::new());
    let shutdown_complete_rx = shutdown_complete_tx.clone();

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer_addr)) => {
                        //println!("Stream: {}, peer_addr: {:?}", Utc::now(), peer_addr);
                        let acceptor = acceptor.clone();
                        let active_connections = active_connections.clone();
                        let shutdown_complete = shutdown_complete_tx.clone();

                        active_connections.fetch_add(1, Ordering::SeqCst);

                        tokio::spawn(async move {
                            match acceptor.accept(stream).await {
                                Ok(tls_stream) => {
                                    handle_stream_connection(tls_stream, peer_addr).await;
                                }
                                Err(e) => {
                                    eprintln!("Stream: Failed to accept TLS connection: {:?}", e);
                                }
                            }
                            if active_connections.fetch_sub(1, Ordering::SeqCst) == 1 {
                                shutdown_complete.notify_one();
                            }
                        });
                    }
                    Err(_e) => {
                        //eprintln!("Stream: Failed to accept connection: {:?}", e);
                        continue;
                    }
                }
            },

            _ = shutdown_receiver.recv() => {
                println!("Stream: Shutdown signal received, stopping accept loop");
                break;
            }
        }
    }

    let timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(timeout);

    tokio::select! {
        _ = shutdown_complete_rx.notified() => {
            println!("Stream: All connections completed gracefully");
        }
        _ = &mut timeout => {
            println!("Stream: Shutdown timeout reached, forcing close");
        }
    }

    drop(listener);
    println!("Stream: Server stopped.");
}
const LENGTH: usize = 4;
async fn handle_stream_connection(mut tls_stream: TlsStream<TcpStream>, _peer_addr: SocketAddr) {
    let mut length_bytes = [0u8; LENGTH];
    while let Ok(_) = tls_stream.read_exact(&mut length_bytes).await {
        // Parse the length from the header
        let msg_length = u32::from_be_bytes(length_bytes) as usize;
        let mut message_body = vec![0u8; msg_length];

        // Read the message body based on the length
        match tls_stream.read_exact(&mut message_body).await {
            Ok(_) => {},
            Err(_e) => {
                //eprintln!("Stream: Error reading message body: {}", e);
                continue;
            }
        }

        // Parse the request from the message body
        let request = match DataServerRequest::from_bytes(&message_body) {
            Ok(req) => req,
            Err(_e) => {
                //eprintln!("Stream: Failed to parse request: {:?}", e);
                continue;
            }
        };
        //println!("{:?}", request);

        // Handle the request and generate a response
        match request {
            DataServerRequest::RegisterStreamer{port, secs, subsec } => {
                initialize_streamer(port, Duration::new(secs, subsec), tls_stream).await;
                //println!("Streamer Registered");
                return;
            },
            _ => eprintln!("Stream: Strategy Did not register a Strategy mode")
        }
    }
    //println!("Stream: TLS connection established with {:?}", peer_addr);
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