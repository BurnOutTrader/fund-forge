use std::hash::Hash;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use iced::{futures, Application, Command, Element, Settings, Theme};
use iced::advanced::Hasher;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::futures::executor::block_on;
use iced::futures::stream::BoxStream;
use iced::Length::Fill;
use iced::subscription::unfold;
use iced::widget::{canvas, container, Text};
use tokio::runtime::Runtime;
use tokio::sync::{Mutex};
use tokio::sync::mpsc::{channel, Receiver};
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, initialize_clients, ConnectionType, PlatformMode};
use ff_standard_lib::server_connections::ConnectionType::Default;
use ff_standard_lib::servers::communications_async::SecondaryDataReceiver;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::strategy_registry::guis::RegistryGuiResponse;
use ff_standard_lib::strategy_registry::RegistrationRequest;
use ff_standard_lib::traits::bytes::Bytes;

#[tokio::main]
async fn main() {
    // Run the async code inside the runtime
    block_on(async {
        let result = initialize_clients(&PlatformMode::MultiMachine).await.unwrap();
    });

    let sender = block_on(get_async_sender(ConnectionType::StrategyRegistry)).unwrap();
    let register_gui = RegistrationRequest::Gui.to_bytes();
    block_on(sender.send(&register_gui)).unwrap();
    let receiver = block_on(get_async_reader(ConnectionType::StrategyRegistry)).unwrap();

    let (gui_sender, gui_receiver) = channel(1000);
    tokio::spawn(async move {
        let mut locked_receiver = receiver.lock().await;
        while let Some(msg) = locked_receiver.receive().await {
            match gui_sender.send(RegistryGuiResponse::from_bytes(&msg).unwrap()).await {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    });

    let flags = Flags {
        receiver: gui_receiver,
    };

    match FundForgeApplication::run(Settings::with_flags(flags)) {
        Ok(_) => {}
        Err(e) => println!("Error running fund forge: {}", e)
    }
}

pub struct Flags {
    receiver: Receiver<RegistryGuiResponse>
}

/// Main application struct
pub struct FundForgeApplication {
    backtest_strategies: Vec<OwnerId>,
    receiver: Arc<Mutex<Receiver<RegistryGuiResponse>>>,
    last_time: i64
}

impl FundForgeApplication {
    pub fn new(receiver: Receiver<RegistryGuiResponse>) -> Self {
        FundForgeApplication{
            backtest_strategies: vec![],
            receiver: Arc::new(Mutex::new(receiver)),
            last_time: 0
        }
    }
}

impl Application for FundForgeApplication {
    type Executor = iced::executor::Default;
    type Message = RegistryGuiResponse;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (FundForgeApplication::new(flags.receiver), Command::none())
    }

    fn title(&self) -> String {
        String::from("Fund Forge")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            RegistryGuiResponse::StrategyEventUpdates(owner, time_stamp, event_slice) => {
                for event in event_slice {
                    match event {
                        StrategyEvent::OrderEvents(_, _) => {}
                        StrategyEvent::DataSubscriptionEvents(_, _, _) => {}
                        StrategyEvent::StrategyControls(_, _, _) => {}
                        StrategyEvent::DrawingToolEvents(_, _, _) => {}
                        StrategyEvent::TimeSlice(_, time, slice) => {
                            //self.notify.notify_one();
                            //self.canvas.update(slice);
                            for data in slice {
                                if data.is_closed() {
                                    println!("{:?}", data.time_utc());
                                    if data.time_utc().timestamp() == self.last_time {
                                        panic!();
                                    }
                                    self.last_time = data.time_utc().timestamp();
                                } else {
                                    //println!("Open bar time {:?}", time);
                                }
                            }
                        }
                        StrategyEvent::ShutdownEvent(_, _) => {}
                        StrategyEvent::WarmUpComplete(_) => {
                            println!("warm up complete");
                        }
                        StrategyEvent::IndicatorEvent(_, _) => {}
                        StrategyEvent::PositionEvents(_) => {}
                    }
                }
            }
            RegistryGuiResponse::ListStrategiesResponse{backtest, live, live_paper} => {
                println!("backtest: {:?}, live: {:?}, live paper: {:?}, ",  backtest, live, live_paper)
            }
            RegistryGuiResponse::Buffer { buffer } => {

            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        Text::new("fund forge").into()
       /*
       let canvas_element = canvas(&self.canvas)
            .width(Fill)
            .height(Fill);

        container(canvas_element)
            .into()
            */
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let messages = iced::Subscription::from_recipe(
            StrategyWindowRecipe {
                registry_reader:  self.receiver.clone(),
            });
        iced::Subscription::batch(vec![messages])
    }
}


pub struct StrategyWindowRecipe {
    registry_reader: Arc<Mutex<Receiver<RegistryGuiResponse>>>,
}

impl Recipe for StrategyWindowRecipe {
    type Output = RegistryGuiResponse;

    fn hash(&self, state: &mut Hasher) {
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        Box::pin(futures::stream::unfold(self.registry_reader.clone(), |receiver| async move {
            let mut locked_receiver = receiver.lock().await;
            locked_receiver.recv().await.map(|message| (message, receiver.clone()))
        }))
    }
}


