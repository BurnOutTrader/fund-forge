use crate::drawing_objects::drawing_tool_enum::DrawingTool;
use crate::standardized_types::subscriptions::DataSubscription;
use ahash::AHashMap;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum DrawingToolEvent {
    Add(DrawingTool),
    Remove(DrawingTool),
    Update(DrawingTool),
    RemoveAll,
}

pub struct DrawingObjectHandler {
    drawing_objects: Arc<RwLock<AHashMap<DataSubscription, Vec<DrawingTool>>>>,
}

impl DrawingObjectHandler {
    pub fn new(drawing_objects: AHashMap<DataSubscription, Vec<DrawingTool>>) -> Self {
        Self {
            drawing_objects: Arc::new(RwLock::new(drawing_objects)),
        }
    }

    pub fn default() -> Self {
        Self {
            drawing_objects: Arc::new(RwLock::new(AHashMap::new())),
        }
    }

    pub async fn drawing_tools(
        &self,
    ) -> RwLockReadGuard<AHashMap<DataSubscription, Vec<DrawingTool>>> {
        self.drawing_objects.read().await
    }

    /// Adds a drawing tool to the strategy.
    /// Drawing objects aren't just Ui objects, they can be interacted with by the engine backend and used for trading signals.
    /// # Arguments
    /// * `drawing_tool` - The drawing tool to add to the strategy.
    pub async fn drawing_tool_add(&self, drawing_tool: DrawingTool) {
        let subscription = drawing_tool.subscription().clone();
        let mut drawing_objects = self.drawing_objects.write().await;
        if !drawing_objects.contains_key(&subscription) {
            drawing_objects.insert(subscription.clone(), Vec::new());
        }
        drawing_objects
            .get_mut(&subscription)
            .unwrap()
            .push(drawing_tool.clone());
        // self.broadcast_strategy_event(StrategyEvent::DrawingToolEvents(self.owner_id.clone(), DrawingToolEvent::Add(drawing_tool), self.time_utc().timestamp())).await;
    }

    /// Removes a drawing tool from the strategy.
    /// Drawing objects aren't just Ui objects, they can be interacted with by the engine backend and used for trading signals.
    /// # Arguments
    /// * `drawing_tool` - The drawing tool to remove from the strategy.
    pub async fn drawing_tool_remove(&self, drawing_tool: DrawingTool) {
        let subscription = drawing_tool.subscription();
        let mut drawing_objects = self.drawing_objects.write().await;
        if drawing_objects.contains_key(&subscription) {
            let index = drawing_objects
                .get_mut(&subscription)
                .unwrap()
                .iter()
                .position(|x| x.id() == drawing_tool.id());
            if let Some(index) = index {
                drawing_objects
                    .get_mut(&subscription)
                    .unwrap()
                    .remove(index);
            }
        }
        //ToDo: we will have external broadcast to the strategy registry for this
        //self.broadcast_strategy_event(StrategyEvent::DrawingToolEvents(self.owner_id.clone(), DrawingToolEvent::Remove(drawing_tool), self.time_utc().timestamp())).await;
    }

    /// Updates a drawing tool in the strategy.
    pub async fn drawing_tool_update(&self, drawing_tool: DrawingTool) {
        let subscription = drawing_tool.subscription().clone();
        let mut drawing_objects = self.drawing_objects.write().await;
        if drawing_objects.contains_key(&subscription) {
            let index = drawing_objects
                .get_mut(&subscription)
                .unwrap()
                .iter()
                .position(|x| x.id() == drawing_tool.id());
            if let Some(index) = index {
                drawing_objects.get_mut(&subscription).unwrap()[index] = drawing_tool.clone();
            }
        }
        //self.broadcast_strategy_event(StrategyEvent::DrawingToolEvents(self.owner_id.clone(), DrawingToolEvent::Update(drawing_tool), self.time_utc().timestamp())).await;
    }

    /// Removes all drawing tools from the strategy.
    pub async fn drawing_tools_remove_all(&self) {
        let mut drawing_objects = self.drawing_objects.write().await;
        drawing_objects.clear();
        //self.broadcast_strategy_event(StrategyEvent::DrawingToolEvents(self.owner_id.clone(), DrawingToolEvent::RemoveAll, self.time_utc().timestamp())).await;
    }
}
