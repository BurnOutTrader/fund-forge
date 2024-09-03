use std::hash::Hash;
use std::intrinsics::mir::Len;
use std::sync::Arc;
use iced::{futures, Application, Command, Element, Theme};
use iced::advanced::Hasher;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::futures::executor::block_on;
use iced::futures::stream::BoxStream;
use iced::Length::Fill;
use iced::widget::{canvas, container};
use tokio::sync::{Mutex, Notify};
use ff_charting::canvas::graph::canvas::SeriesCanvas;
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, initialize_clients, ConnectionType, PlatformMode};
use ff_standard_lib::servers::communications_async::SecondaryDataReceiver;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::strategy_registry::guis::RegistryGuiResponse;
use ff_standard_lib::strategy_registry::RegistrationRequest;
use ff_standard_lib::traits::bytes::Bytes;

#[tokio::main]
async fn main() {
    initialize_clients(&PlatformMode::MultiMachine).await?;

    iced::application(
        "Fund Forge",
        Default::default()
    ).run()
}



/// Main application struct
pub struct FundForgeApplication {
    //canvas: SeriesCanvas,
}

impl FundForgeApplication {
    pub fn new() -> Self {
        FundForgeApplication{

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
        String::from("Series Canvas Chart")
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
                            println!("{:?}" slice)
                        }
                        StrategyEvent::ShutdownEvent(_, _) => {}
                        StrategyEvent::WarmUpComplete(_) => {}
                        StrategyEvent::IndicatorEvent(_, _) => {}
                        StrategyEvent::PositionEvents(_) => {}
                    }
                }
            }
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let canvas_element = canvas(&self.canvas)
            .width(Fill)
            .height(Fill);

        container(canvas_element)
            .into()
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
            locked_receiver.receive().await.map(|message| (message, receiver.clone()))
        }))
    }
}


