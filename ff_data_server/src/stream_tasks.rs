use std::sync::{Arc};
use std::time::Duration;
use dashmap::DashMap;
use futures::future::join_all;
use lazy_static::lazy_static;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_rustls::server::TlsStream;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::bytes_trait::Bytes;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;
use ff_standard_lib::StreamName;

lazy_static! {
    static ref STREAM_RECEIVERS: DashMap<u16 , Arc<DashMap<DataSubscription ,broadcast::Receiver<BaseDataEnum>>>> = DashMap::new();
    static ref STREAM_TASKS: DashMap<u16 , (JoinHandle<()>, JoinHandle<()>)> = DashMap::new();
}

pub fn shutdown_stream_tasks() {
    for stream in STREAM_TASKS.iter() {
        deregister_streamer(stream.key());
    }
}

pub async fn initialize_streamer(stream_name: StreamName, buffer: Duration, stream: TlsStream<TcpStream>) {
    let map = Arc::new(DashMap::new());
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
        sender_ref.insert(subscription, receiver);
    }
}

pub async fn unsubscribe_stream(stream_name: &StreamName, subscription: &DataSubscription) {
    if let Some(sender_ref) = STREAM_RECEIVERS.get(stream_name) {
        sender_ref.remove(subscription);
    }
}
const LENGTH: usize = 4;
const BATCH_SIZE: usize = 20;

pub async fn stream_handler(
    stream_name: StreamName,
    buffer: Duration,
    mut stream: TlsStream<TcpStream>,
    stream_receivers: Arc<DashMap<DataSubscription, broadcast::Receiver<BaseDataEnum>>>,
) -> (JoinHandle<()>, JoinHandle<()>) {
    let (data_sender, mut data_receiver) = mpsc::channel::<TimeSlice>(10);

    let receiver_task = tokio::spawn(async move {
        loop {
            if stream_receivers.is_empty() {
                sleep(Duration::from_millis(500)).await;
                continue;
            }
            let futures = stream_receivers
                .iter()
                .map(|entry| {
                    let key = entry.key().clone();
                    let data_sender = data_sender.clone();
                    let stream_receivers = stream_receivers.clone();
                    tokio::spawn(async move {
                        if let Some(mut entry) = stream_receivers.get_mut(&key) {
                            process_receiver(entry.value_mut(), data_sender).await;
                        }
                    })
                })
                .collect::<Vec<_>>();

            join_all(futures).await;
        }
    });

    let time_slice_task = tokio::spawn(async move {
        let mut time_slice = TimeSlice::new();
        let mut interval = tokio::time::interval(buffer);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !time_slice.is_empty() {
                        let bytes = time_slice.to_bytes();
                        let length = (bytes.len() as u32).to_be_bytes();
                        let mut prefixed_msg = Vec::with_capacity(LENGTH + bytes.len());
                        prefixed_msg.extend_from_slice(&length);
                        prefixed_msg.extend_from_slice(&bytes);

                        if let Err(e) = stream.write_all(&prefixed_msg).await {
                            eprintln!("Error writing to stream: {}", e);
                            break;
                        }
                        time_slice.clear();
                    }
                }
                Some(slice) = data_receiver.recv() => {
                    time_slice.extend(slice);
                }
            }
        }
        println!("Stream handler for {} has shut down", stream_name);
    });

    (receiver_task, time_slice_task)
}

async fn process_receiver(
    rx: &mut broadcast::Receiver<BaseDataEnum>,
    data_sender: mpsc::Sender<TimeSlice>,
) {
    let mut time_slice = TimeSlice::new();
    let mut count = 0;
    while let Ok(base_data_enum) = rx.try_recv() {
        time_slice.add(base_data_enum);
        count += 1;
        if count >= BATCH_SIZE {
            if data_sender.send(time_slice).await.is_err() {
                return; // Main task has been dropped
            }
            time_slice = TimeSlice::new();
            count = 0;
        }
    }
    if !time_slice.is_empty() {
        let _ = data_sender.send(time_slice).await;
    }
}