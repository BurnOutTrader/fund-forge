use iced::Rectangle;
use iced::widget::canvas::Frame;
use crate::canvas::graph::models::time_scale::TimeScale;
use crate::canvas::graph::state::ChartState;

#[derive(Debug, Clone)]
pub enum XScale {
    Time(TimeScale),
    Linear,
}

impl XScale {

    pub fn increment_factor(&self) -> i64 {
        match self {
            XScale::Time(time_scale) => {
                time_scale.resolution.as_seconds()
            }
            XScale::Linear => {
                1
            }
        }
    }

    pub fn scale_height(&self) -> f32 {
        match self {
            XScale::Time(time_scale) => {
                time_scale.scale_height()
            }
            _ => panic!("Not implemented")
        }
    }

    pub fn draw(&self, frame: &mut Frame, view: &ChartState, bounds: &Rectangle)  {
        if view.x_end - view.x_start <= 0 || view.x_start > view.x_end {
            return;
        }
        match self {
            XScale::Time(time_grid) => {
                time_grid.draw_scale(frame, view, bounds)
            }
            _ => panic!("Not implemented")
        }
    }
}