use iced::mouse::Button;
use iced::Point;
use crate::chart_canvas::graph::enums::areas::ChartAreas;
use crate::chart_canvas::graph::state::ChartState;

#[derive(Clone, PartialEq, Copy)]
pub struct Click {
    pub time_pressed: i64,
    pub position: Point,
    pub location: ChartAreas,
    pub button: Button,
}

impl Click {
    pub fn new(view: &ChartState, time_pressed: i64, position: Point, button: Button) -> Click {
        Click {
            time_pressed,
            position,
            button,
            location: click_location(view, position),
        }
    }
}

pub fn click_location(view: &ChartState, position: Point) -> ChartAreas {
    if view.drawing_area.contains(position) {
        return ChartAreas::DrawingArea;
    } else if view.y_scale_area.contains(position) {
        return ChartAreas::PriceScale
    } else if view.x_scale_area.contains(position) {
        return ChartAreas::DateScale
    } else if  view.data_area.contains(position) {
        return ChartAreas::DataArea
    }
    ChartAreas::None
}