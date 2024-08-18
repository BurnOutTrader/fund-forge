use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use tokio::sync::RwLock;
use ff_charting::drawing_tool_enum::DrawingTool;
use ff_standard_lib::standardized_types::data_server_messaging::FundForgeError;
use ff_standard_lib::standardized_types::orders::orders::OrderUpdateEvent;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use ff_standard_lib::standardized_types::time_slices::TimeSlice;


/// Used to determine if a strategy takes certain event inputs from the Ui.
/// A trader can still effect the strategy via the broker, but the strategy will not take any input from the Ui unless in SemiAutomated mode.
/// In replays this allows us to signal when the event was created by the strategy or the Ui.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Copy)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum StrategyInteractionMode {
    /// Takes subscription and orders input from the ui as well as programmatically
    SemiAutomated,
    /// Takes no input from the Ui
    Automated,
    
}

/*#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Copy)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum OrderEvent {
    Create
}*/

/// All strategies can be sent or received by the strategy or the UI.
/// This enum server multiple purposes.
/// All `utc timestamp (i64)` Should be in Utc time
/// All messages broadcast using a [DataBroadcaster](ff_common_library::streams::broadcasting::DataBroadcaster), which allows messages to be sent to multiple subscribers depending on the intended [BroadcastDirection](ff_common_library::streams::broadcasting::BroadcastDirection).
/// # In Strategies
/// 1. `BroadcastDirection::External` forwards event updates to the Strategy Register Service. So they can be processed by a remote Ui or Strategy.
/// 2. `BroadcastDirection::Internal` and an incoming event is received from a broker or data vendor it can be sent to the `strategy.broadcaster` to be processed by the strategy using the passed in receiver.
/// 3. `BroadcastDirection::All` data needs to be shared both internally and externally, it can be forwarded by the `strategy.broadcaster` to both remote and internal subscribers.
/// # In the Strategy Register Service
/// 1. Events will be recorded as BTreeMaps with the `utc timestamp` as the key and Vec<StrategyEvent> as the value.
/// 2. Strategies can be replayed by the replay engine by simply iterating over the BTreeMap and sending the strategies to the strategy.
/// # Benefits
/// 1. Allows copy trading.
/// 2. Allows inter-strategy relations from separate containers or machines.
/// 3. Allows for remote Ui connections.
/// 4. Allows for multiple strategies to be run in parallel or the design of multi-strategy code bases operating as a program.
/// 5. Allows recording the strategies of a strategy for later playback.
/// # Warning
/// It is prudent not to broadcast every piece of data to every subscriber, for example passing a message to inform the strategy about an event that it created has the potential to create an infinite feedback loop.
/// In the context of adding a subscriber, if we were to pass this event internally, it would inform the strategy that a new subscriber has been added, which would then send the same message again, and so on.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum StrategyEvent {
    /// Communicates order-related strategies between the UI, strategy, and brokerage connections.
    ///
    /// # Parameters
    /// - `OwnerId`: The unique identifier of the owner (strategy or UI).
    /// - `OrderEvent`: Details of the order event.
    /// - `EventForwarderType`: Specifies the type of event forwarder (Strategy or UI).
    OrderEvents(OwnerId, OrderUpdateEvent),

    /// Allows for the subscription and un-subscription of data feeds remotely.
    ///
    /// # Parameters
    /// - `OwnerId`: The unique identifier of the owner (strategy or UI).
    /// - `DataSubscriptionEvent`: The subscription event details.
    /// - `EventForwarderType`: Specifies the type of event forwarder (Strategy or UI).
    /// - `i64`: A timestamp indicating when the event was created.
    DataSubscriptionEvents(OwnerId, DataSubscriptionEvent, i64),

    /// Enables remote control of strategy operations.
    ///
    /// # Parameters
    /// - `OwnerId`: The unique identifier of the owner (strategy or UI).
    /// - `StrategyControls`: The control command to be executed.
    /// - `EventForwarderType`: Specifies the type of event forwarder (Strategy or UI).
    /// - `i64`: A timestamp indicating when the event was created.
    StrategyControls(OwnerId, StrategyControls, i64),

    /// Facilitates interaction with drawing tools between the UI and strategies.
    ///
    /// # Parameters
    /// - `OwnerId`: The unique identifier of the owner (strategy or UI).
    /// - `DrawingToolEvent`: The drawing tool event details.
    /// - `i64`: A timestamp indicating when the event was created.
    DrawingToolEvents(OwnerId, DrawingToolEvent, i64),


    /// Records time slices for playback, excluding backtests.
    ///
    /// # Parameters
    /// - `OwnerId`: The unique identifier of the owner (strategy).
    /// - `TimeSlice`: The time slice data.
    TimeSlice(OwnerId, TimeSlice),

    ShutdownEvent(OwnerId, String)
}

impl StrategyEvent {
    pub fn get_owner_id(&self) -> OwnerId {
        match self {
            StrategyEvent::OrderEvents(owner_id, _) => owner_id.clone(),
            StrategyEvent::DataSubscriptionEvents(owner_id, _, _) => owner_id.clone(),
            StrategyEvent::StrategyControls(owner_id, _, _) => owner_id.clone(),
            StrategyEvent::DrawingToolEvents(owner_id, _, _) => owner_id.clone(),
            StrategyEvent::TimeSlice(owner_id, _) => owner_id.clone(),
            StrategyEvent::ShutdownEvent(owner_id, _) => owner_id.clone()
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.extend_from_slice(b"\n\n");
        vec.into()
    }

    pub fn from_bytes(archived: &[u8]) -> Result<StrategyEvent, FundForgeError> {
        let archived_without_delimiter = &archived[..archived.len() - 2];
        match rkyv::from_bytes::<StrategyEvent>(archived_without_delimiter) { //Ignore this warning: Trait `Deserialize<StrategyEvent, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277]
            Ok(message) => Ok(message),
            Err(e) => {
                Err(FundForgeError::ClientSideErrorDebug(e.to_string()))
            }
        }
    }
}


/// The event that is sent to the Strategy Register Service when a strategy is shutdown programmatically.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum ShutdownEvent {
    Error(String),
    Success(String)
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum DrawingToolEvent {
    Add(DrawingTool),
    Remove(DrawingTool),
    Update(DrawingTool),
    RemoveAll
}




#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum DataSubscriptionEvent {
    Subscribed(DataSubscription),
    Unsubscribed(DataSubscription)
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum PlotEvent {
    Add(Plot),
    Remove(Plot),
    Update(Plot)
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum Plot {
    PLACEHOLDER
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
/// Used to remotely control_center the strategy
pub enum StrategyControls {
    /// To continue a strategy that is paused and allow it to continue trading.
    Continue,
    /// The strategy is paused, it will still monitor data feeds but will not be able to trade.
    /// Useful for strategies that take time to warm up but need to be deployed quickly.
    Pause,
    /// Used to stop strategies.
    Stop,
    /// Used to start strategies.
    Start,
    /// Used to set the delay time, to speed up or slow down backtests
    Delay(Option<u64>)
}

