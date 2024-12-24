use ff_standard_lib::gui_types::settings::GraphElementSettings;
use crate::chart_canvas::graph::enums::plots::PlotEnum;

#[derive(Debug, PartialEq, Clone)]
pub struct XYPlot {
    time_local: i64,
    value: f64,
    settings: GraphElementSettings,
    size: f32,
    style: PlotEnum,
    id: String,
}