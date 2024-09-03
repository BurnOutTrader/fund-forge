use std::hash::Hash;
use std::sync::Arc;
use iced::{futures, Application, Command, Element, Settings, Theme};
use iced::advanced::Hasher;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::futures::executor::block_on;
use iced::futures::stream::BoxStream;
use iced::Length::Fill;
use iced::widget::{canvas, container, Text};
use tokio::runtime::Runtime;
use tokio::sync::{Mutex};
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, initialize_clients, ConnectionType, PlatformMode};
use ff_standard_lib::servers::communications_async::SecondaryDataReceiver;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::strategy_registry::guis::RegistryGuiResponse;
use ff_standard_lib::strategy_registry::RegistrationRequest;
use ff_standard_lib::traits::bytes::Bytes;


fn main() {
    let runtime = Runtime::new().unwrap();
    // Run the async code inside the runtime
    runtime.block_on(async {
        let result = initialize_clients(&PlatformMode::MultiMachine).await.unwrap();
    });
    match FundForgeApplication::run(Settings::default()) {
        Ok(_) => {}
        Err(e) => println!("Error running fund forge: {}", e)
    }
}



/// Main application struct
pub struct FundForgeApplication {
    backtest_strategies: Vec<OwnerId>
}

impl FundForgeApplication {
    pub fn new() -> Self {
        FundForgeApplication{
            backtest_strategies: vec![]
        }
    }
}

impl Application for FundForgeApplication {
    type Executor = iced::executor::Default;
    type Message = RegistryGuiResponse;
    type Theme = Theme;
    type Flags = ();

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (FundForgeApplication::new(), Command::none())
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
                        StrategyEvent::TimeSlice(_, slice) => {
                            //self.notify.notify_one();
                            //self.canvas.update(slice);
                            println!("{:?}", slice)
                        }
                        StrategyEvent::ShutdownEvent(_, _) => {}
                        StrategyEvent::WarmUpComplete(_) => {}
                        StrategyEvent::IndicatorEvent(_, _) => {}
                        StrategyEvent::PositionEvents(_) => {}
                    }
                }
            }
            RegistryGuiResponse::ListStrategiesResponse{backtest, live, live_paper} => {
                println!("backtest: {:?}, live: {:?}, live paper: {:?}, ",  backtest, live, live_paper)
            }
            RegistryGuiResponse::Subscribed(_, _) => {}
            RegistryGuiResponse::Unsubscribed(_) => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        Text::new("fund forge").into()
       /* let canvas_element = canvas(&self.canvas)
            .width(Fill)
            .height(Fill);

        container(canvas_element)
            .into()*/
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let sender = block_on(get_async_sender(ConnectionType::StrategyRegistry)).unwrap();
        let register_gui = RegistrationRequest::Gui.to_bytes();
        block_on(sender.send(&register_gui)).unwrap();
        let receiver = block_on(get_async_reader(ConnectionType::StrategyRegistry)).unwrap();
        let messages = iced::Subscription::from_recipe(
            StrategyWindowRecipe {
                registry_reader:  receiver,
            });
        iced::Subscription::batch(vec![messages])
    }
}


pub struct StrategyWindowRecipe {
    registry_reader: Arc<Mutex<SecondaryDataReceiver>>
}

impl Recipe for StrategyWindowRecipe {
    type Output = RegistryGuiResponse;

    fn hash(&self, state: &mut Hasher) {
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        Box::pin(futures::stream::unfold(self.registry_reader.clone(), |receiver| async move {
            let mut locked_receiver = receiver.lock().await;
            locked_receiver.receive().await.map(|message| (RegistryGuiResponse::from_bytes(&message).unwrap(), receiver.clone()))
        }))
    }
}


