use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use tokio::sync::{Mutex, RwLock};
use crate::strategy_registry::{RegistrationRequest, RegistrationResponse};
use crate::strategy_registry::strategies::StrategyResponse;
use crate::servers::communications_async::{SecondaryDataReceiver, SecondaryDataSender};
use crate::servers::communications_sync::SynchronousCommunicator;
use crate::standardized_types::data_server_messaging::{FundForgeError};
use crate::standardized_types::{OwnerId, TimeString};
use crate::standardized_types::strategy_events::{EventTimeSlice, StrategyEvent};
use crate::strategy_registry::handle_gui::handle_gui;
use crate::strategy_registry::handle_strategies::handle_strategies;
use crate::traits::bytes::Bytes;


/// this is used when launching in single machine
pub async fn registry_manage_async_requests(sender: Arc<SecondaryDataSender>, receiver: Arc<Mutex<SecondaryDataReceiver>>) {
    tokio::spawn(async move {
        let receiver = receiver.clone();
        let sender = sender;
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
                RegistrationRequest::Strategy(owner) => {
                    handle_strategies(owner.clone(), sender, receiver).await;
                    break 'register_loop
                }
                RegistrationRequest::Gui => {
                    handle_gui(sender, receiver).await;
                    break 'register_loop
                }
                _ => {}
            };
        }
    });
}
