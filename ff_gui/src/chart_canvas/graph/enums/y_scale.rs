use iced::Rectangle;
use iced::widget::canvas::Frame;
use crate::chart_canvas::graph::models::price_scale::PriceScale;
use crate::chart_canvas::graph::state::ChartState;

#[derive(Debug, Clone)]
pub enum YScale {
    Price(PriceScale),
}

impl YScale {
    pub fn draw(&self, frame: &mut Frame, view: &ChartState, bounds: &Rectangle)  {
        if view.y_high  - view.y_low <= 0.0 {
            return;
        }
        match self {
            YScale::Price(price_scale) => {
                price_scale.draw_scale(frame, view, bounds)
            }
        }
    }

    pub fn last_value(&mut self, last_price: f64) {
        match self {
            YScale::Price(price_scale) => {
                price_scale.last_price = last_price;
            }
        }
    }

    pub fn last_comparison_value(&mut self, last_open_price: f64) {
        match self {
            YScale::Price(price_scale) => {
                price_scale.last_open_price = Some(last_open_price);
            }
        }
    }

    pub fn scale_width(&self) -> f32 {
        match self {
            YScale::Price(price_scale) => {
                price_scale.scale_width()
            }
        }
    }

    pub fn logorithmic(&self) -> bool {
        match self {
            YScale::Price(price_scale) => {
                price_scale.logarithmic
            }
        }
    }
}

