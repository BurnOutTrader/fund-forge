use std::sync::Arc;
use crate::standardized_types::strategy_events::{
    StrategyControls, StrategyInteractionMode,
};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use crate::servers::internal_broadcaster::StaticInternalBroadcaster;

pub struct InteractionHandler {
    control_state: RwLock<StrategyControls>,
    is_warmed_up: RwLock<bool>,
    ///delay to add to market replay data feed to slow down backtests
    replay_delay_ms: RwLock<Option<u64>>,
    broadcaster: Arc<StaticInternalBroadcaster<StrategyControls>>
}

impl InteractionHandler {
    pub fn new(
        replay_delay_ms: Option<u64>,
        _interaction_mode: StrategyInteractionMode,
    ) -> InteractionHandler {
        InteractionHandler {
            control_state: RwLock::new(StrategyControls::Continue),
            is_warmed_up: RwLock::new(false),
            replay_delay_ms: RwLock::new(replay_delay_ms),
            broadcaster: Arc::new(StaticInternalBroadcaster::new())
        }
    }

    pub async fn set_warmup_complete(&self) {
        *self.is_warmed_up.write().await = true;
    }

    pub async fn subscribe(&self, name: String, sender: Sender<StrategyControls>) {
        self.broadcaster.subscribe(name, sender);
    }

    pub async fn set_control_state(&self, control_state: StrategyControls) {
        *self.control_state.write().await = control_state;
    }

    pub async fn control_state(&self) -> StrategyControls {
        self.control_state.read().await.clone()
    }

    pub async fn set_replay_delay_ms(&self, delay: Option<u64>) {
        *self.replay_delay_ms.write().await = delay;
    }

    pub async fn replay_delay_ms(&self) -> Option<u64> {
        *self.replay_delay_ms.read().await
    }

    pub async fn process_controls(&self) -> bool {
        loop {
            match self.control_state().await {
                StrategyControls::Start => return false,
                StrategyControls::Continue => return false,
                StrategyControls::Pause => tokio::time::sleep(Duration::from_millis(500)).await,
                StrategyControls::Stop => return true,
                StrategyControls::Delay(ms) => {
                    self.set_replay_delay_ms(ms).await;
                    self.set_control_state(StrategyControls::Continue).await;
                    return false;
                }
            }
        }
    }
}
