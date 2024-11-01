use std::sync::Arc;
use ahash::AHashMap;
use crate::messages::data_server_messaging::{DataServerRequest, StreamRequest};
use crate::standardized_types::enums::StrategyMode;
use crate::strategies::client_features::connection_settings::client_settings::initialise_settings;
use crate::strategies::client_features::connection_types::ConnectionType;
use crate::strategies::client_features::request_handler::StrategyRequest;
use crate::strategies::client_features::request_handler;
use crate::strategies::handlers::subscription_handler::SubscriptionHandler;

pub(crate) async fn live_subscription_handler(
    mode: StrategyMode,
    subscription_handler: Arc<SubscriptionHandler>
) {
    if mode == StrategyMode::Backtest {
        return;
    }

    let settings_map = Arc::new(initialise_settings().unwrap());
    let mut subscription_update_channel = subscription_handler.subscribe_primary_subscription_updates();

    let settings_map_ref = settings_map.clone();
    println!("Handler: Start Live handler");
    tokio::task::spawn(async move {
        let mut current_subscriptions = subscription_handler.primary_subscriptions().await.clone();
        {
            let mut subscribed = vec![];
            println!("Handler: {:?}", current_subscriptions);
            for subscription in &*current_subscriptions {
                let request = DataServerRequest::StreamRequest {
                    request: StreamRequest::Subscribe(subscription.clone())
                };
                let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                let connection_type = match settings_map_ref.contains_key(&connection) {
                    true => connection,
                    false => ConnectionType::Default
                };
                if !subscribed.contains(&connection_type) {
                    let register = StrategyRequest::OneWay(connection_type.clone(), DataServerRequest::Register(mode.clone()));
                    request_handler::send_request(register).await;
                    subscribed.push(connection_type.clone());
                }
                let request = StrategyRequest::OneWay(connection_type, request);
                request_handler::send_request(request).await;
            }
        }
        while let Ok(updated_subscriptions) = subscription_update_channel.recv().await {
            let mut requests_map = AHashMap::new();
            if current_subscriptions != updated_subscriptions {
                for subscription in &updated_subscriptions {
                    if !current_subscriptions.contains(&subscription) {
                        let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                        let connection_type = match settings_map_ref.contains_key(&connection) {
                            true => connection,
                            false => ConnectionType::Default
                        };
                        let request = DataServerRequest::StreamRequest { request: StreamRequest::Subscribe(subscription.clone())};
                        if !requests_map.contains_key(&connection_type) {
                            requests_map.insert(connection_type, vec![request]);
                        } else {
                            requests_map.get_mut(&connection_type).unwrap().push(request);
                        }
                    }
                }
                for subscription in &*current_subscriptions {
                    if !updated_subscriptions.contains(&subscription) {
                        let connection = ConnectionType::Vendor(subscription.symbol.data_vendor.clone());
                        let connection_type = match settings_map_ref.contains_key(&connection) {
                            true => connection,
                            false => ConnectionType::Default
                        };
                        let request = DataServerRequest::StreamRequest { request: StreamRequest::Unsubscribe(subscription.clone())};

                        if !requests_map.contains_key(&connection_type) {
                            requests_map.insert(connection_type, vec![request]);
                        } else {
                            requests_map.get_mut(&connection_type).unwrap().push(request);
                        }
                    }
                }
                for (connection, requests) in requests_map {
                    for request in requests {
                        let request = StrategyRequest::OneWay(connection.clone(), request);
                        request_handler::send_request(request).await;
                    }
                }
                current_subscriptions = updated_subscriptions.clone();
            }
        }
    });
}
