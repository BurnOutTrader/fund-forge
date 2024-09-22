use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use crate::strategy_registry::RegistrationRequest;
use crate::traits::bytes::Bytes;
use std::sync::Arc;
use tokio::sync::Mutex;

/// this is used when launching in single machine
pub async fn registry_manage_async_requests(
    _sender: Arc<SecondaryDataSender>,
    receiver: Arc<Mutex<SecondaryDataReceiver>>,
) {
    tokio::spawn(async move {
        let receiver = receiver.clone();
        //let sender = sender;
        let binding = receiver.clone();
        let mut listener = binding.lock().await;
        'register_loop: while let Some(data) = listener.receive().await {
            let request = match RegistrationRequest::from_bytes(&data) {
                Ok(request) => request,
                Err(e) => {
                    println!("Failed to parse request: {:?}", e);
                    continue;
                }
            };
            match request {
                RegistrationRequest::Strategy(_owner, _mode, _subscriptions) => {
                    //handle_strategies(owner.clone(), sender, receiver, mode.clone()).await;
                    //let gui_response = RegistryGuiResponse::StrategyAdded(owner, mode, subscriptions);
                    //broadcast(gui_response.to_bytes()).await;
                    break 'register_loop;
                }
                RegistrationRequest::Gui => {
                    //handle_gui(sender, receiver).await;
                    break 'register_loop;
                }
            };
        }
    });
}
