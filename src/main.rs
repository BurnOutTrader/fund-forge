use tokio::sync::{mpsc,RwLock};
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, initialize_clients, ConnectionType, PlatformMode};
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::traits::bytes::Bytes;
use std::sync::Arc;
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::strategy_registry::guis::{GuiRequest, RegistryGuiResponse};
use ff_standard_lib::strategy_registry::{RegistrationRequest};
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, };
use ff_standard_lib::standardized_types::subscriptions::{CandleType, DataSubscription};


#[tokio::main]
async fn main() {
    initialize_clients(&PlatformMode::MultiMachine).await.unwrap();
    let async_sender = get_async_sender(ConnectionType::StrategyRegistry).await.unwrap();
    let register_event = RegistrationRequest::Gui.to_bytes();
    let registry_reader = get_async_reader(ConnectionType::StrategyRegistry).await.unwrap();
    async_sender.send(&register_event).await.unwrap();
    registry_reader.lock().await.receive().await.unwrap();
    let subscribe_event = GuiRequest::Subscribe(String::from("test")).to_bytes();
    async_sender.send(&subscribe_event).await.unwrap();
    let (series_sender, series_receiver) = mpsc::channel(1000);

    let mut is_warmed_up = Arc::new(RwLock::new(false));


    let subscription = DataSubscription::new_custom("AUD-USD".to_string(), DataVendor::Test, Resolution::Minutes(15), BaseDataType::Candles, MarketType::Forex, CandleType::CandleStick);
    let bars : Vec<Candle> = Vec::new();
    let warm = is_warmed_up.clone();
    tokio::spawn(async move {
        let warm = warm.clone();
        let mut receiver = registry_reader.lock().await;
        let subscription = subscription.clone();
        'listener_loop: while let Some(msg) = receiver.receive().await {
            let response = RegistryGuiResponse::from_bytes(&msg).unwrap();
            //series_sender.send(response.clone()).await.unwrap();
            match &response {
                RegistryGuiResponse::StrategyEventUpdates(owner, time, events) => {
                    for event in events {
                        match event {
                            StrategyEvent::OrderEvents(_, _) => {}
                            StrategyEvent::DataSubscriptionEvents(_, event, _) => {
                                println!("received DataSubscriptionEvents: {:?}", event);
                            }
                            StrategyEvent::StrategyControls(_, _, _) => {}
                            StrategyEvent::DrawingToolEvents(_, _, _) => {}
                            StrategyEvent::TimeSlice(_, slice) => {
                                for data in slice {
                                    if data.subscription() != subscription {
                                        continue
                                    }

                                    match data {
                                        BaseDataEnum::Candle(candle) => {
                                            if !*warm.read().await {
                                                if candle.is_closed {
                                                    println!("{}", candle);
                                                }
                                            } else {
                                                let _send = series_sender.send(response.clone()).await;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            StrategyEvent::ShutdownEvent(_, event) => {
                                //todo, make strategy engine wait for a response on async registry channel before shutting down.
                                println!("received ShutdownEvent: {:?}", event);
                                break 'listener_loop
                            }
                            StrategyEvent::WarmUpComplete(_) => {
                                *warm.write().await = true;
                                println!("received WarmUpComplete");
                            }
                            StrategyEvent::IndicatorEvent(_, indicator_event) => {
                                match indicator_event {
                                    IndicatorEvents::IndicatorAdded(added_event) => {
                                        println!("Indicator Added: {:?}", added_event);
                                    }
                                    IndicatorEvents::IndicatorRemoved(removed_event) => {
                                        println!("Indicator Removed: {:?}", removed_event);
                                    }
                                    IndicatorEvents::IndicatorTimeSlice(slice_event) => {
                                        // we can see our auto manged indicator values for here.
                                        for indicator_values in slice_event {
                                            println!("{}: \n {:?}", indicator_values.name(), indicator_values.values());
                                        }
                                    }
                                    IndicatorEvents::Replaced(_) => {}
                                }
                            }
                        }
                    }
                }
                RegistryGuiResponse::ListStrategiesResponse(_) => {
                    println!("received ListStrategiesResponse response: {:?}", response);
                }
                RegistryGuiResponse::Subscribed(owner, buffer) => {
                    println!("received Subscribed response: {:?}", owner);
                }
                RegistryGuiResponse::Unsubscribed(owner) => {
                    println!("received unSubscribe response: {:?}", response);
                }
            }
        }
    });
}
