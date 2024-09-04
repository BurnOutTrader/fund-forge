use crate::chart_canvas::graph::models::price_scale::PriceScale;
use std::collections::BTreeMap;
use std::sync::{Arc};
use chrono::{DateTime, Utc};
use chrono_tz::Tz::UTC;
use iced::Rectangle;
use iced_graphics::core::Color;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use ff_standard_lib::app::settings::GraphElementSettings;
use ff_standard_lib::drawing_objects::drawing_tool_enum::DrawingTool;
use ff_standard_lib::standardized_types::enums::StrategyMode;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::strategy_events::StrategyEvent;
use ff_standard_lib::standardized_types::subscriptions::{DataSubscription, DataSubscriptionEvent, SymbolName};
use crate::chart_canvas::graph::canvas::ChartCanvas;
use crate::chart_canvas::graph::enums::x_scale::XScale;
use crate::chart_canvas::graph::enums::y_scale::YScale;
use crate::chart_canvas::graph::models::crosshair::CrossHair;
use crate::chart_canvas::graph::models::time_scale::TimeScale;

pub struct StrategyView {
    owner_id: OwnerId,

    charts: Arc<RwLock<BTreeMap<DataSubscription, ChartCanvas>>>,

    /// The number of chart canvases we are displaying
    display_zones: Arc<RwLock<u8>>,

    drawing_tools: Arc<RwLock<BTreeMap<SymbolName, DrawingTool>>>,

    /// last updated is the actual time not the strategy time, so we can update from buffers
    last_updated: Arc<RwLock<DateTime<Utc>>>,

    is_active: Arc<RwLock<bool>>,

    strategy_mode: StrategyMode
}

impl StrategyView {
    pub fn new(owner_id: OwnerId, mode: StrategyMode, subscriptions: Vec<DataSubscription>, receiver: Receiver<StrategyEvent>) -> Self {
        let mut charts: BTreeMap<DataSubscription, ChartCanvas> = BTreeMap::new();
        for sub in subscriptions {
            let x_scale = XScale::Time(TimeScale::new(GraphElementSettings::default(), sub.resolution.clone()));
            let y_scale = YScale::Price(PriceScale::default());
            let canvas = ChartCanvas::new(BTreeMap::new(), Color::new(1.0, 1.0, 1.0, 1.0), x_scale,
                                          Rectangle::default(), y_scale, vec![], CrossHair::default(), UTC);
            charts.insert(sub, canvas);
        }
        let view = StrategyView {
            owner_id,
            charts: Arc::new(RwLock::new(charts)),
            display_zones: Arc::new(RwLock::new(0)),
            drawing_tools: Default::default(),
            last_updated: Arc::new(RwLock::new(Utc::now())),
            is_active: Arc::new(RwLock::new(false)),
            strategy_mode: mode,
        };

        view.update_gui_response(receiver);
        view
    }

    pub fn update_gui_response(&self, receiver: Receiver<StrategyEvent>) {
        let mut receiver = receiver;
        let charts = self.charts.clone();
        let drawing_tools = self.drawing_tools.clone();
        let last_updated = self.last_updated.clone();
        let is_active = self.is_active.clone();
        tokio::spawn(async move {
            let charts = charts.clone();
            let drawing_tools = drawing_tools.clone();
            let last_updated = last_updated.clone();
            let is_active = is_active.clone();
            let mut receiver = receiver;
            while let Some(message) = receiver.recv().await {
                match message {
                    StrategyEvent::OrderEvents(_, _) => {}
                    StrategyEvent::DataSubscriptionEvents(_, events, _) => {
                        for event in events {
                            match event {
                                DataSubscriptionEvent::Subscribed(subscription) => {
                                    if !charts.read().await.contains_key(&subscription) {
                                        let x_scale = XScale::Time(TimeScale::new(GraphElementSettings::default(), subscription.resolution.clone()));
                                        let y_scale = YScale::Price(PriceScale::default());
                                        let canvas = ChartCanvas::new(BTreeMap::new(), Color::new(1.0, 1.0, 1.0, 1.0), x_scale,
                                                                      Rectangle::default(), y_scale, vec![], CrossHair::default(), UTC);

                                        charts.write().await.insert(subscription, canvas);
                                    }
                                }
                                DataSubscriptionEvent::Unsubscribed(_) => {}
                                DataSubscriptionEvent::Failed(_) => {}
                            }
                        }
                    }
                    StrategyEvent::StrategyControls(_, _, _) => {}
                    StrategyEvent::DrawingToolEvents(_, _, _) => {}
                    StrategyEvent::TimeSlice(_, time, time_slice) => {
                        for base_data in time_slice {
                            let mut charts = charts.write().await;
                            if let Some(chart_view) = charts.get_mut(&base_data.subscription()) {
                                chart_view.update(&base_data)
                            }
                        }
                    }
                    StrategyEvent::ShutdownEvent(_, _) => {}
                    StrategyEvent::WarmUpComplete(_) => {}
                    StrategyEvent::IndicatorEvent(_, _) => {}
                    StrategyEvent::PositionEvents(_) => {}
                }
            }
        });
    }
}