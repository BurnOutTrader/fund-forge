use std::sync::Arc;
use tokio::sync::Mutex;
use ff_standard_lib::communicators::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use ff_standard_lib::messages::registry_messages::guis::{GuiRequest, RegistryGuiResponse};
use ff_standard_lib::traits::bytes::Bytes;
use crate::handle_strategies::{get_backtest_connected_strategies, get_events_buffer, get_live_connected_strategies, get_live_paper_connected_strategies, send_subscriber, subscribe, unsubscribe};

pub async fn handle_gui(
    sender: Arc<SecondaryDataSender>,
    receiver: Arc<Mutex<SecondaryDataReceiver>>,
) {
    tokio::spawn(async move {
        let receiver = receiver.clone();
        let id = subscribe(sender).await;
        let binding = receiver.clone();
        let mut listener = binding.lock().await;
         while let Some(data) = listener.receive().await {
            tokio::spawn(async move {
                let request = match GuiRequest::from_bytes(&data) {
                    Ok(request) => request,
                    Err(_) => return,
                };
                match request {
                    GuiRequest::ListAllStrategies => {
                        tokio::spawn(async move {
                            let live = get_live_connected_strategies().await;
                            let backtest = get_backtest_connected_strategies().await;
                            let live_paper = get_live_paper_connected_strategies().await;
                            send_subscriber(id, RegistryGuiResponse::ListStrategiesResponse { live, backtest, live_paper }.to_bytes()).await;
                        });
                    }
                    GuiRequest::RequestBuffers => {
                        let _buffers = get_events_buffer().await;
                        //send_subscriber(id, RegistryGuiResponse::Buffer {buffer: buffers}.to_bytes()).await;
                    }
                };
            });
        }
        unsubscribe(id).await;
        println! {"Gui Disconnected"}
    });
}
