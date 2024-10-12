use std::sync::{Arc};
use std::time::Duration;
use ahash::AHashMap;
use dashmap::DashMap;
use futures::stream::{StreamExt, FuturesUnordered};
use lazy_static::lazy_static;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_rustls::server::TlsStream;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::StreamName;

lazy_static! {
    static ref STREAM_RECEIVERS: DashMap<u16 , Arc<RwLock<AHashMap<DataSubscription ,broadcast::Receiver<BaseDataEnum>>>>> = DashMap::new();
    static ref STREAM_TASKS: DashMap<u16 , (JoinHandle<()>, JoinHandle<()>)> = DashMap::new();
}

pub fn shutdown_stream_tasks() {
    for stream in STREAM_TASKS.iter() {
        deregister_streamer(stream.key());
    }
}

pub async fn initialize_streamer(stream_name: StreamName, buffer: Duration, stream: TlsStream<TcpStream>) {
    let map = Arc::new(RwLock::new(AHashMap::new()));
    let handles = stream_handler(stream_name, buffer, stream, map.clone()).await;
    STREAM_TASKS.insert(stream_name, handles);
    STREAM_RECEIVERS.insert(stream_name, map);
}

pub fn deregister_streamer(stream_name: &StreamName) {
    STREAM_RECEIVERS.remove(stream_name);
    if let Some((_, handles)) = STREAM_TASKS.remove(stream_name) {
        let (receiver_task, timeslice_task) = handles;
        receiver_task.abort();
        timeslice_task.abort();
    }
}

pub async fn subscribe_stream(stream_name: &StreamName, subscription: DataSubscription, receiver: broadcast::Receiver<BaseDataEnum>) {
    if let Some(sender_ref) = STREAM_RECEIVERS.get(stream_name) {
        let mut write_ref = sender_ref.write().await;
        write_ref.insert(subscription, receiver);
    }
}

pub async fn unsubscribe_stream(stream_name: &StreamName, subscription: &DataSubscription) {
    if let Some(sender_ref) = STREAM_RECEIVERS.get(stream_name) {
        let mut write_ref = sender_ref.write().await;
        write_ref.remove(subscription);
    }
}

const LENGTH: usize = 4;
pub async fn stream_handler(
    stream_name: StreamName,
    buffer: Duration,
    mut stream: TlsStream<TcpStream>,
    stream_receivers: Arc<RwLock<AHashMap<DataSubscription, broadcast::Receiver<BaseDataEnum>>>>,
) -> (JoinHandle<()>, JoinHandle<()>) {

        let (data_sender, mut data_receiver) = mpsc::channel::<BaseDataEnum>(500);  // Adjust buffer size as needed

    let receiver_task = tokio::spawn(async move {
        loop {
            let receivers = stream_receivers.read().await;
            if receivers.is_empty() {
                drop(receivers);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }

            let mut futures = FuturesUnordered::new();
            for (_, rx) in receivers.iter() {
                let mut rx = rx.resubscribe();
                futures.push(async move { rx.recv().await });
            }
            drop(receivers);

            while let Some(result) = futures.next().await {
                if let Ok(base_data_enum) = result {
                    if data_sender.send(base_data_enum).await.is_err() {
                        return;  // Main task has been dropped
                    }
                }
            }
        }
    });
    let time_slice_task = tokio::spawn(async move {
        let mut time_slice = TimeSlice::new();
        let mut interval = interval(buffer);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let bytes = time_slice.to_bytes();
                    let length: [u8; LENGTH] = (bytes.len() as u32).to_be_bytes();
                    let mut prefixed_msg = Vec::new();
                    prefixed_msg.extend_from_slice(&length);
                    prefixed_msg.extend_from_slice(&bytes);

                    if let Err(e) = stream.write_all(&prefixed_msg).await {
                        eprintln!("Error writing to stream: {}", e);
                        break;
                    }
                    time_slice = TimeSlice::new();
                }
                Some(base_data_enum) = data_receiver.recv() => {
                    time_slice.add(base_data_enum);
                }
            }
        }
        println!("Stream handler for {} has shut down", stream_name);
    });
    (receiver_task, time_slice_task)
}