use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use crate::strategy_registry::guis::{GuiRequest, RegistryGuiResponse};
use crate::strategy_registry::handle_strategies::{
    clear_subscriptions, get_connected_strategies, get_events_buffer,
    subscribe_to_strategy, unsubscribe_from_strategy,
};
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
        let sender = sender.clone();
        let binding = receiver.clone();
        let mut listener = binding.lock().await;
         while let Some(data) = listener.receive().await {
            let sender = sender.clone();
            tokio::spawn(async move {
                let request = match GuiRequest::from_bytes(&data) {
                    Ok(request) => request,
                    Err(_) => return,
                };
                match request {
                    GuiRequest::ListAllStrategies => {
                        let strategies = get_connected_strategies().await;
                        match strategies.is_empty() {
                            true => {}
                            false => match sender
                                .send(
                                    &RegistryGuiResponse::ListStrategiesResponse(strategies)
                                        .to_bytes(),
                                )
                                .await
                            {
                                Ok(_) => {}
                                Err(_) => {}
                            },
                        }
                    }
                    GuiRequest::Subscribe(owner) => {
                        subscribe_to_strategy(owner.clone(), sender.clone()).await;
                        let buffer = get_events_buffer(&owner).await;
                        match sender
                            .send(
                                &RegistryGuiResponse::Subscribed(owner.clone(), buffer).to_bytes(),
                            )
                            .await
                        {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    GuiRequest::Unsubscribe(owner) => {
                        unsubscribe_from_strategy(owner).await;
                    }
                };
            });
        }
        clear_subscriptions().await;
        println! {"Gui Disconnected"}
    });
}
