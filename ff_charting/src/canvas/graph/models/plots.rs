use ff_standard_lib::app::settings::GraphElementSettings;
use crate::canvas::graph::enums::plots::PlotStyle;

#[derive(Debug, PartialEq, Clone)]
pub struct XYPlot {
    time_local: i64,
    value: f64,
    settings: GraphElementSettings,
    size: f32,
    style: PlotStyle,
    id: String,
}