use std::time::Duration;
use tokio::sync::RwLock;
use crate::messages::strategy_events::{StrategyControls, StrategyInteractionMode};

pub struct InteractionHandler {
    control_state: RwLock<StrategyControls>,
    ///delay to add to market replay data feed to slow down backtests
    replay_delay_ms: RwLock<Option<u64>>,
}

impl InteractionHandler {
    pub fn new(replay_delay_ms: Option<u64>, interaction_mode: StrategyInteractionMode) -> InteractionHandler {
        InteractionHandler {
            control_state: RwLock::new(StrategyControls::Continue),
            replay_delay_ms: RwLock::new(replay_delay_ms),
        }
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

    pub(crate) async fn process_controls(&self) -> bool {
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