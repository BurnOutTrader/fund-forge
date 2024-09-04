use chrono_tz::Tz;
use iced::{Color, Point, Rectangle, Size};
use iced::widget::canvas::Frame;
use iced_graphics::geometry::{Path, Stroke};
use ff_standard_lib::standardized_types::base_data::candle::Candle;
use crate::chart_canvas::graph::state::ChartState;

/// Trait used for all objects that plot on time series chart, used to return geometry which matched the bounds and number of objects, thus geometry can be easily cached.
pub trait TimeSeriesGraphElements {
    fn time_stamp_local(&self, time_zone: &Tz) -> i64;
    fn value(&self) -> f64;
    fn draw_object(&self, frame: &mut Frame, view: &ChartState, bounds: &Rectangle, logarithmic: bool, time_zone: &Tz);
}

impl TimeSeriesGraphElements for Candle {
    /// The unadjusted local timestamp, for candles or bars this is the opening time for most brokers and we do not use self.data_vendor.time_closed() to make adjustments when drawing.
    fn time_stamp_local(&self, time_zone: &Tz) -> i64 {
        self.time_local(time_zone).timestamp()
    }

    fn value(&self) -> f64 {
        self.close
    }

    fn draw_object(&self, frame: &mut Frame, view: &ChartState, bounds: &Rectangle, logarithmic: bool, time_zone: &Tz) {
        let local_time = self.time_stamp_local(time_zone);
        if local_time < view.x_start || local_time> view.x_end {
            return ;
        }

        let true_x = match view.calculate_x(&local_time) {
            Some(true_x) => true_x,
            None => return
        };

        if true_x < 0.0 || true_x > bounds.width {
            return
        }

        let high = match view.calculate_y(self.high, logarithmic) {
            Some(high) => high,
            None => return
        };

        let low = match view.calculate_y(self.low, logarithmic) {
            Some(low) => low,
            None => return
        };

        let open = match view.calculate_y(self.open, logarithmic) {
            Some(open) => open,
            None => return
        };

        let close = match view.calculate_y(self.close, logarithmic) {
            Some(close) => close,
            None => return
        };

        let bar_width = view.x_object_space_width();

        let wick_width = bar_width / 50.0;

        let wick = Path::rectangle(Point::new(true_x, high), Size::new(wick_width.max(1.0), low - high));


        let color = if close > open {
            Color::from_rgb(0.0, 0.7, 0.0)
        } else if close < open {
            Color::from_rgb(0.7, 0.0, 0.0)
        } else {
            Color::from_rgb(0.0, 0.0, 0.4)
        };

        // Draw the candlestick on the same frame
        frame.fill(&wick, color);
        frame.stroke(&wick, Stroke::default().with_color(color).with_width(wick_width.max(1.0)));

        let (body_top, body_bottom) = if self.close > self.open { (close, open) } else { (open, close) };
        let body_height = body_bottom - body_top;
        let body = Path::rectangle(Point::new(true_x - bar_width * 0.4, body_top), Size::new(bar_width * (1.0 - view.space_percent), body_height.max(1.0)));

        frame.fill(&body, color);
    }
}