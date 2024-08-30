use tokio::sync::{mpsc, Mutex, Notify, RwLock};
use tokio::task;
use chrono_tz::Australia;
use ff_charting::canvas::graph::canvas::{ChartApp, GraphFlags, SeriesCanvas};
use ff_charting::canvas::graph::enums::x_scale::XScale;
use ff_charting::canvas::graph::enums::y_scale::YScale;
use ff_standard_lib::indicators::indicator_handler::IndicatorEvents;
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, initialize_clients, ConnectionType, PlatformMode};
use ff_standard_lib::servers::communications_async::SendError;
use ff_standard_lib::servers::registry_request_handlers::{EventRequest, EventResponse};
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::traits::bytes::Bytes;
use iced::{Application, Settings, Command, Element, Color, Theme};
use iced::widget::canvas::{Canvas};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use chrono::{DateTime, NaiveDate, Utc}; // Replace with your own modules
use iced::window;
use tokio::time::sleep;
use ff_charting::canvas::graph::models::data::SeriesData;
use ff_charting::canvas::graph::models::price_scale::PriceScale;
use ff_charting::canvas::graph::models::time_scale::TimeScale;
use ff_standard_lib::apis::vendor::DataVendor;
use ff_standard_lib::app::settings::GraphElementSettings;
use ff_standard_lib::consolidators::consolidator_enum::ConsolidatorEnum;
use ff_standard_lib::helpers::converters::convert_to_utc;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::base_data_type::BaseDataType;
use ff_standard_lib::standardized_types::base_data::history::range_data;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::{MarketType, Resolution, StrategyMode};
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;

#[tokio::main]
async fn main() {
    initialize_clients(&PlatformMode::MultiMachine).await.unwrap();
    let registry_reader = get_async_reader(ConnectionType::StrategyRegistry).await.unwrap();
    let asnc_sender = get_async_sender(ConnectionType::StrategyRegistry).await.unwrap();

    let events = vec![EventRequest::RegisterGui.to_bytes(), EventRequest::Subscribe(String::from("test")).to_bytes()];

    for event in events {
        match asnc_sender.send(&event).await {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    let mut is_warmed_up = Arc::new(RwLock::new(false));
    let (series_sender, series_receiver) = mpsc::channel(1000);
    let notify = Arc::new(Notify::new());
    let notified = notify.clone();
    let warm_up_bars: Arc<RwLock<BTreeMap<i64, Vec<SeriesData>>>> = Arc::new(RwLock::new(BTreeMap::new()));
    let bars = warm_up_bars.clone();

    let subscription = DataSubscription::new("AUD-CAD".to_string(), DataVendor::Test, Resolution::Minutes(3), BaseDataType::Candles, MarketType::Forex);
    let warm = is_warmed_up.clone();
    tokio::spawn(async move {
        let notified = notified.clone();
        let warm = warm.clone();
        let mut receiver = registry_reader.lock().await;
        'listener_loop: while let Some(msg) = receiver.receive().await {
            let response = EventResponse::from_bytes(&msg).unwrap();
            //series_sender.send(response.clone()).await.unwrap();
            match &response {
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
                                let mut bars = warm_up_bars.write().await;
                                for data in slice {
                                    if data.subscription() != subscription {
                                        continue
                                    }
                                    if !*warm.write().await {
                                        match data {
                                            BaseDataEnum::Candle(candle) => {
                                                println!("{}", candle);
                                                if !*warm.read().await {
                                                    match bars.get_mut(&data.time_utc().timestamp()) {
                                                        Some(mut vec) => {
                                                            vec.push(SeriesData::CandleStick(candle.clone()))
                                                        }
                                                        None => {
                                                            bars.insert(data.time_utc().timestamp(), vec![SeriesData::CandleStick(candle.clone())]);
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        series_sender.send(response.clone()).await;
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
                EventResponse::ListStrategiesResponse(_) => {
                    println!("received ListStrategiesResponse response: {:?}", response);
                }
                EventResponse::Subscribed(_) => {
                    println!("received Subscribed response: {:?}", response);
                }
            }
            //notified.notified().await;
        }
    });

    let mut warmed = is_warmed_up.read().await.clone();
    while !warmed {
        sleep(Duration::from_millis(10));
        warmed = is_warmed_up.read().await.clone();
    }
    
    println!("bars count {}", bars.read().await.len());
    let bars = bars.write().await.clone();
    
    //let consolidator = ConsolidatorEnum::warmup(consolidator, convert_to_utc(to, Australia::Sydney), StrategyMode::Backtest).await;
    let canvas = SeriesCanvas::new("test".to_string(), bars.clone(), Default::default(), XScale::Time(TimeScale::new(Default::default(),
                                               Resolution::Minutes(3))), Default::default(), YScale::Price(PriceScale::default()), vec![], Default::default(), Australia::Sydney);
    let flags = GraphFlags {
        receiver: Arc::new(Mutex::new(series_receiver)),
        canvas,
        notify: notify.clone()
    };
    let settings = Settings {
        window: window::Settings {
            resizable: true,
            ..window::Settings::default()
        },
        flags,
        ..Settings::default()
    };

    // Run the Iced application
    match ChartApp::run(settings) {
        Ok(_) => {}
        Err(_) => {}
    }
}
