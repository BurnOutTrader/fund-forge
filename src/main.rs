use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, initialize_clients, ConnectionType, PlatformMode};
use ff_standard_lib::servers::communications_async::SendError;
use ff_standard_lib::servers::registry_request_handlers::{EventRequest, EventResponse};
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::traits::bytes::Bytes;

#[tokio::main]
async fn main() {
    initialize_clients(&PlatformMode::MultiMachine).await.unwrap();
    let registry_reader = get_async_reader(ConnectionType::StrategyRegistry).await.unwrap();
    let asnc_sender = get_async_sender(ConnectionType::StrategyRegistry).await.unwrap();
    let subscribe_event = EventRequest::Subscribe(String::from("test")).to_bytes();
    match asnc_sender.send(&subscribe_event).await {
        Ok(_) => {}
        Err(_) => {}
    }

    let mut receiver = registry_reader.lock().await;
    while let Some(msg) = receiver.receive().await {
        let response = EventResponse::from_bytes(&msg).unwrap();
        match response {
            EventResponse::StrategyEventUpdates(_, events) => {
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
                                if data.is_closed() {
                                    println!("{}", data);
                                }
                            }
                        }
                        StrategyEvent::ShutdownEvent(_, event) => {
                            //todo, make strategy engine wait for a response on async registry channel before shutting down.
                            println!("received ShutdownEvent: {:?}", event);
                        }
                        StrategyEvent::WarmUpComplete(_) => {
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
            EventResponse::ListStrategiesResponse(_) => {
                println!("received ListStrategiesResponse response: {:?}", response);
            }
            EventResponse::Subscribed(_) => {
                println!("received Subscribed response: {:?}", response);
            }
        }
    }
}
