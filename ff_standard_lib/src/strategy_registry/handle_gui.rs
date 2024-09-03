use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender, SendError};
use crate::strategy_registry::guis::{GuiRequest, RegistryGuiResponse};
use crate::strategy_registry::handle_strategies::{broadcast, get_backtest_connected_strategies, get_events_buffer, get_live_connected_strategies, get_live_paper_connected_strategies, send_subscriber, subscribe, unsubscribe};
use crate::strategy_registry::RegistrationResponse;
use crate::traits::bytes::Bytes;
use std::sync::Arc;
use tokio::sync::Mutex;

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
                        let buffers = get_events_buffer().await;
                        send_subscriber(id, RegistryGuiResponse::Buffer {buffer: buffers}.to_bytes()).await;
                    }
                };
            });
        }
        unsubscribe(id).await;
        println! {"Gui Disconnected"}
    });
}
