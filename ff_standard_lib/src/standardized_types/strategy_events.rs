use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use crate::drawing_objects::drawing_object_handler::DrawingToolEvent;
use crate::standardized_types::data_server_messaging::FundForgeError;
use crate::standardized_types::orders::orders::OrderUpdateEvent;
use crate::standardized_types::subscriptions::{DataSubscriptionEvent};
use crate::standardized_types::time_slices::TimeSlice;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::ser::Serializer;
use rkyv::{AlignedVec, Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use rkyv::validation::CheckTypeError;
use rkyv::validation::validators::DefaultValidator;
use rkyv::vec::ArchivedVec;
use crate::indicators::events::IndicatorEvents;
use crate::standardized_types::accounts::position::PositionUpdateEvent;

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug, Copy, Ord, PartialOrd, Eq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum StrategyEventType {
    OrderEvents,
    DataSubscriptionEvents,
    StrategyControls,
    DrawingToolEvents,
    TimeSlice,
    ShutdownEvent,
    WarmUpComplete,
    IndicatorEvent,
    PositionEvents,
}

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
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum StrategyEvent {
    /// Communicates order-related strategies between the UI, strategy, and brokerage connections.
    ///
    /// # Parameters
    /// - `OrderEvent`: Details of the order event.
    OrderEvents(OrderUpdateEvent),

    /// Allows for the subscription and un-subscription of data feeds remotely.
    ///
    /// # Parameters
    /// - `DataSubscriptionEvent`: The subscription event details.
    DataSubscriptionEvent(DataSubscriptionEvent),

    /// Enables remote control of strategy operations.
    ///
    /// # Parameters
    /// - `StrategyControls`: The control command to be executed.
    StrategyControls(StrategyControls),

    /// Facilitates interaction with drawing tools between the UI and strategies.
    ///
    /// # Parameters
    /// - `DrawingToolEvent`: The drawing tool event details.
    DrawingToolEvents(DrawingToolEvent),

    /// Contains strategy BaseDataEnum's as TimeSlice.
    ///
    /// # Parameters
    /// - `TimeSlice`: The time slice data.
    TimeSlice(TimeSlice),

    ShutdownEvent(String),

    WarmUpComplete,

    IndicatorEvent(IndicatorEvents),


    PositionEvents(PositionUpdateEvent)
}

impl StrategyEvent {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec = rkyv::to_bytes::<_, 256>(self).unwrap();
        vec.extend_from_slice(b"\n\n");
        vec.into()
    }

    pub fn from_bytes(archived: &[u8]) -> Result<StrategyEvent, FundForgeError> {
        let archived_without_delimiter = &archived[..archived.len() - 2];
        match rkyv::from_bytes::<StrategyEvent>(archived_without_delimiter) {
            //Ignore this warning: Trait `Deserialize<StrategyEvent, SharedDeserializeMap>` is not implemented for `ArchivedUiStreamResponse` [E0277]
            Ok(message) => Ok(message),
            Err(e) => Err(FundForgeError::ClientSideErrorDebug(e.to_string())),
        }
    }

    pub fn vec_to_aligned(events: Vec<StrategyEvent>) -> AlignedVec {
        // Create a new serializer
        let mut serializer = AllocSerializer::<20971520>::default();

        // Serialize the Vec<QuoteBar>
        serializer.serialize_value(&events).unwrap();

        // Get the serialized bytes
        let vec = serializer.into_serializer().into_inner();
        vec
    }

    pub fn get_type(&self) -> StrategyEventType {
        match self {
            StrategyEvent::OrderEvents(_) => StrategyEventType::OrderEvents,
            StrategyEvent::StrategyControls(_) => StrategyEventType::StrategyControls,
            StrategyEvent::DrawingToolEvents(_) => StrategyEventType::DrawingToolEvents,
            StrategyEvent::TimeSlice(_) => StrategyEventType::TimeSlice,
            StrategyEvent::ShutdownEvent(_) => StrategyEventType::ShutdownEvent,
            StrategyEvent::WarmUpComplete => StrategyEventType::WarmUpComplete,
            StrategyEvent::IndicatorEvent(_) => StrategyEventType::IndicatorEvent,
            StrategyEvent::PositionEvents(_) => StrategyEventType::PositionEvents,
            StrategyEvent::DataSubscriptionEvent(_) => StrategyEventType::DataSubscriptionEvents
        }
    }

    pub fn from_array_bytes(data: &Vec<u8>) -> Result<Vec<StrategyEvent>, CheckTypeError<ArchivedVec<ArchivedStrategyEvent>, DefaultValidator>> {
        let archived_event = match rkyv::check_archived_root::<Vec<StrategyEvent>>(&data[..]) {
            Ok(data) => data,
            Err(e) => {
                return Err(e);
            }
        };

        // Assuming you want to work with the archived data directly, or you can deserialize it further
        Ok(archived_event.deserialize(&mut rkyv::Infallible).unwrap())
    }
}

/// The event that is sent to the Strategy Register Service when a strategy is shutdown programmatically.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum ShutdownEvent {
    Error(String),
    Success(String),
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum PlotEvent {
    Add(Plot),
    Remove(Plot),
    Update(Plot),
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum Plot {
    PLACEHOLDER,
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
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
    Delay(Option<u64>),
    /// Use Strings to set custom commands to the strategy
    Custom(String),
    /// Send bytes over TCP for larger more complex commands that can be deserialized to concrete types by a u64 identifier
    CustomBytes(u64, Vec<u8>)
}

#[derive(Clone, PartialEq, Debug)]
pub struct StrategyEventBuffer {
    /// Events stored with their DateTimes in order
    events: Vec<(DateTime<Utc>, StrategyEvent)>,
    /// Keep a record of event index by event type so we can seperate events.
    events_by_type: BTreeMap<StrategyEventType, Vec<u64>>,
}

impl StrategyEventBuffer {
    pub fn new() -> Self {
        StrategyEventBuffer {
            events: Vec::new(),
            events_by_type: BTreeMap::new(),
        }
    }

    // Adds an event with a given time maintaining the order of the event slice so that when we iterate earliest occurring events will be presented first
    pub fn add_event(&mut self, time: DateTime<Utc>, event: StrategyEvent) {
        // Insert event into the main list
        self.events.push((time, event.clone()));

        // Sort the events by time (stable sort keeps order for equal times)
        self.events.sort_by_key(|(time, _)| *time);

        // Instead of finding the index after sorting, we could assume the event may have shifted
        // and directly locate it using binary search.
        let sorted_event_index = self
            .events
            .binary_search_by_key(&time, |(t, _)| *t)
            .unwrap_or_else(|_| panic!("Newly added event not found after sorting"));

        // Insert the sorted index into the type map and keep it sorted by time
        let entry = self
            .events_by_type
            .entry(event.get_type()) // Assuming StrategyEvent has a `get_type` method
            .or_insert_with(Vec::new);

        // Use binary search to find the correct insertion position
        let insertion_position = entry
            .binary_search_by_key(&time, |&idx| self.events[idx as usize].0)
            .unwrap_or_else(|pos| pos); // Find the correct insertion position
        entry.insert(insertion_position, sorted_event_index as u64);
    }

    // Iterate over the sorted events by time
    pub fn iter(&self) -> impl Iterator<Item = &(DateTime<Utc>, StrategyEvent)> {
        self.events.iter()
    }

    // Returns an iterator over events of a given type, sorted by time (borrowed version)
    pub fn get_events_by_type(
        &self,
        event_type: StrategyEventType,
    ) -> impl Iterator<Item = &(DateTime<Utc>, StrategyEvent)> {
        self.events_by_type
            .get(&event_type)
            .into_iter()
            .flat_map(move |indices| {
                indices.iter().map(move |&idx| &self.events[idx as usize])
            })
    }

    // Returns a Vec of events of a given type, sorted by time (owned version)
    // This fn does not remove the events but instead clones them.
    // If you are using this fn to move certain events to another function, be careful that you do not double handle events, ie react to the same event twice
    pub fn get_owned_events_by_type(
        &self,
        event_type: StrategyEventType,
    ) -> Vec<(DateTime<Utc>, StrategyEvent)> {
        self.events_by_type
            .get(&event_type)
            .map_or_else(Vec::new, |indices| {
                indices
                    .iter()
                    .map(|&idx| self.events[idx as usize].clone()) // Clone to return owned data
                    .collect()
            })
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.events_by_type.clear();
    }
}

#[cfg(test)]
mod tests {

}
