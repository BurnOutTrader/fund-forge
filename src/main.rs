use std::cell::RefCell;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::Arc;
use iced::{futures, Application, Command, Element, Settings, Theme};
use iced::advanced::Hasher;
use iced::advanced::subscription::{EventStream, Recipe};
use iced::futures::executor::block_on;
use iced::futures::stream::BoxStream;
use iced::widget::{Text};
use tokio::sync::{Mutex};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use ff_gui::strategy_view::strategy_view::StrategyView;
use ff_standard_lib::server_connections::{get_async_reader, get_async_sender, initialize_clients, ConnectionType, PlatformMode};
use ff_standard_lib::servers::communications_async::SecondaryDataSender;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::strategy_registry::guis::{GuiRequest, RegistryGuiResponse};
use ff_standard_lib::strategy_registry::RegistrationRequest;
use ff_standard_lib::traits::bytes::Bytes;

#[tokio::main]
async fn main() {
   /* // Run the async code inside the runtime
    block_on(async {
        let result = initialize_clients(&PlatformMode::MultiMachine).await.unwrap();
    });

    let sender = block_on(get_async_sender(ConnectionType::StrategyRegistry)).unwrap();
    let register_gui = RegistrationRequest::Gui.to_bytes();
    block_on(sender.send(&register_gui)).unwrap();
    let request_strategies = GuiRequest::ListAllStrategies.to_bytes();
    block_on(sender.send(&request_strategies)).unwrap();
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
        registry_sender: sender
    };

    match FundForgeApplication::run(Settings::with_flags(flags)) {
        Ok(_) => {}
        Err(e) => println!("Error running fund forge: {}", e)
    }*/
}

pub struct Flags {
    receiver: Receiver<RegistryGuiResponse>,
    registry_sender: Arc<SecondaryDataSender>
}

/// Main application struct
pub struct FundForgeApplication {
    receiver: Arc<Mutex<Receiver<RegistryGuiResponse>>>,
    last_time: i64,
    strategy_views: BTreeMap<OwnerId, StrategyView>,
    strategy_view_senders: BTreeMap<OwnerId, Sender<StrategyEvent>>,
    registry_sender: Arc<SecondaryDataSender>
}

impl FundForgeApplication {
    pub fn new(receiver: Receiver<RegistryGuiResponse>,  registry_sender: Arc<SecondaryDataSender>) -> Self {
        FundForgeApplication{
            receiver: Arc::new(Mutex::new(receiver)),
            last_time: 0,
            strategy_views: BTreeMap::new(),
            strategy_view_senders: BTreeMap::new(),
            registry_sender
        }
    }

    fn add_strategy(&mut self, owner_id: OwnerId, strategy_mode: StrategyMode, subscriptions: Vec<DataSubscription>) {
        let (sender, receiver) = channel(1000);
        let strategy_view = StrategyView::new(owner_id.clone(), strategy_mode, subscriptions, receiver);
        self.strategy_view_senders.insert(owner_id.clone(), sender);
        self.strategy_views.insert(owner_id, strategy_view);
    }

    fn update_strategies_map(&mut self, backtest: Vec<OwnerId>, live: Vec<OwnerId>, live_paper: Vec<OwnerId>) {
       /* for owner_id in backtest {
            if !self.strategy_views.contains_key(&owner_id) {
                self.add_strategy(owner_id, StrategyMode::Backtest)
            }
        }
        for owner_id in live {
            if !self.strategy_views.contains_key(&owner_id) {
                self.add_strategy(owner_id, StrategyMode::Live)
            }
        }
        for owner_id in live_paper {
            if !self.strategy_views.contains_key(&owner_id) {
                self.add_strategy(owner_id, StrategyMode::LivePaperTrading)
            }
        }*/
    }
}

impl Application for FundForgeApplication {
    type Executor = iced::executor::Default;
    type Message = RegistryGuiResponse;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (FundForgeApplication::new(flags.receiver, flags.registry_sender), Command::none())
    }

    fn title(&self) -> String {
        String::from("Fund Forge")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            RegistryGuiResponse::ListStrategiesResponse{backtest, live, live_paper} => {

            },
            RegistryGuiResponse::StrategyAdded(owner_id, strategy_mode, subscriptions) => {
                self.add_strategy(owner_id, strategy_mode, subscriptions);
            },
            RegistryGuiResponse::StrategyDisconnect(owner_id) => {

            }
            RegistryGuiResponse::StrategyEventUpdates(..) => {

            }

            RegistryGuiResponse::Buffer { buffer } => {

            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        Text::new("fund forge").into()
       /*
       let canvas_element = chart_canvas(&self.chart_canvas)
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


