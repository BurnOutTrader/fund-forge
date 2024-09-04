use iced::{Color, Point, Rectangle, Size};
use iced::widget::canvas;
use iced_graphics::geometry::{LineDash, Path, Stroke, Style};
use std::collections::BTreeMap;
use ff_standard_lib::app::settings::GraphElementSettings;
use crate::chart_canvas::graph::models::crosshair::color_from_template;
use crate::chart_canvas::graph::models::data::SeriesData;
use crate::chart_canvas::graph::state::ChartState;

#[derive(Debug, Clone)]
pub struct PriceScale {
    pub settings: GraphElementSettings,
    pub last_price_settings: GraphElementSettings,
    pub decimal_precision:  usize,
    pub logarithmic: bool,
    pub last_price: f64,
    pub last_open_price: Option<f64>,
}

impl Default for PriceScale {
    fn default() -> Self {
        Self {
            settings: GraphElementSettings::light_mode_settings(),
            last_price_settings: GraphElementSettings::light_mode_settings(),
            last_price: 0.0,
            last_open_price: None,
            logarithmic: false,
            decimal_precision: 5,
        }
    }
}

const LASTPRICEDASHLENGTH: f32 = 2.0;
const LASTPRICEGAPLENGTH: f32 = 2.0;
impl PriceScale {
    pub fn dark_mode_settings(decimal_precision: usize, logorithmic: bool ) -> Self {
        Self {
            settings: GraphElementSettings::dark_mode_settings(),
            last_price_settings: GraphElementSettings::dark_mode_settings(),
            last_price: 0.0,
            last_open_price: None,
            logarithmic: logorithmic,
            decimal_precision,
        }
    }
    pub fn light_mode_settings(decimal_precision: usize, logorithmic: bool ) -> Self {
        Self {
            settings: GraphElementSettings::light_mode_settings(),
            last_price_settings: GraphElementSettings::light_mode_settings(),
            last_price: 0.0,
            last_open_price: None,
            logarithmic: logorithmic,
            decimal_precision,
        }
    }


    pub fn last_open_price(data: &BTreeMap<i64, Vec<SeriesData>>) -> Option<f64> {
        let mut last_open_price: Option<f64> = None;
        for (_, data) in data.iter() {
            for data in data.iter() {
                match data {
                    SeriesData::CandleStick(candle_stick) => {
                        last_open_price = Some(candle_stick.open);
                    },
                }
            }
        }
        last_open_price
    }

    pub fn last_price(data: &BTreeMap<i64, Vec<SeriesData>>) -> Option<f64> {
        let mut last_price: Option<f64> = None;
        for (_, data) in data.iter() {
            for data in data.iter() {
                match data {
                    SeriesData::CandleStick(candle_stick) => {
                        last_price = Some(candle_stick.close);
                    },
                }
            }
        }
        last_price
    }

    pub fn scale_width(&self) -> f32 {
        let max_price_str = format!("{:.*}", self.decimal_precision,self. last_price);
        match self.settings.text_settings.show {
            true => max_price_str.len() as f32 * (self.settings.text_settings.size / 1.5) + 5.0,
            false => 0.0,
        }
    }

    /// Draws the price scale and the last price line
    pub fn draw_scale(
        &self,
        frame: &mut canvas::Frame,
        view: &ChartState,
        bounds: &Rectangle,
    ) {

        // calulate and return the path to the text box, if a text box is created we don't want to draw text under it or on top of it
        // Normalize the price to find its position on the chart
        let last_price_y = match view.calculate_y(self.last_price, self.logarithmic) {
            Some(y) => y,
            None => return
        };

        let last_price_color= match self.last_open_price {
            Some(last_open_price) => {
                if self.last_price > last_open_price {
                    Color::from_rgb(0.0, 0.8, 0.0)
                } else if self.last_price < last_open_price {
                    Color::from_rgb(0.8, 0.0, 0.0)
                } else {
                    Color::from_rgb(0.0, 0.0, 0.8)
                }
            },
            None => Color::from_rgb(0.0, 0.0, 0.8)
        };

        // Format the last price text with the specified decimal precision
        let mut last_price_text="".to_string();

        // Determine whether to create the rectangle based on the text settings visibility
        let last_p_rectangle: Option<Rectangle> = if last_price_y.is_infinite() || last_price_y.is_nan() {
            None
        } else  {
            match self.last_price_settings.text_settings.show {
                true => {
                    last_price_text = format!("{:.*}", self.decimal_precision, self.last_price);
                    let text_box_width = self.scale_width();
                    let text_box_height = self.last_price_settings.text_settings.size * 1.5;
                    // Adjusted calculation for the Y position of the text box
                    let text_box_y = last_price_y - text_box_height / 2.0;
                    // Create the rectangle at the desired position with the calculated size
                    Some(Rectangle::new(
                        Point::new(view.y_scale_area.x, text_box_y), // Adjusted to position the text box within the chart area
                        Size::new(text_box_width, text_box_height)
                    ))
                },
                false => {
                    None
                }
            }
        };


        let grid_interval = self.calculate_grid_interval(view);

        // Align the lowest_price to the nearest appropriate grid line start
        let mut aligned_start = (view.y_low / grid_interval ).floor() * grid_interval;
        if aligned_start + grid_interval - view.y_low > grid_interval as f64 * 0.5 {
            aligned_start += grid_interval; // Adjust if the difference is significant
        }

        // Ensure the starting line is a multiple of 5 or 10, adjusting if necessary
        aligned_start = adjust_to_nearest_five_or_zero(aligned_start, grid_interval);

        let mut first_line = aligned_start;
        //println!("first_line: {}", first_line);
        let mut num_intervals = ((view.y_high - view.y_low) / grid_interval).ceil(); // Ensure covering the whole range

        let last_line = first_line + (grid_interval * num_intervals);
        let last_line_y = view.calculate_y(last_line, self.logarithmic);
        if let Some(last_line_y) = last_line_y {
            if last_line_y < view.y_scale_area.y -(view.y_scale_area.y - view.y_scale_area.height) / num_intervals as f32  {
                num_intervals += 1.0;
                first_line -= grid_interval;
            }
        }

        //println!("last_line: {}", last_line);

        //calulate the space in iced length between the start and the bottom of the price scale
        let first_line_y = view.calculate_y(first_line, self.logarithmic);
        if let Some(first_line_y) = first_line_y {
            if first_line_y  >  view.x_scale_area.y - (view.y_scale_area.y - view.y_scale_area.height) / num_intervals as f32 {
                num_intervals += 1.0;
                first_line -= grid_interval;
            }
        }


        let bounds = view.drawing_area;

        let lower_bounds = view.y_scale_area.y - view.y_scale_area.height;
        let upper_bounds = bounds.y;

        for i in 0..=num_intervals as usize {
            let price = first_line + (grid_interval * i as f64);

            if price as f32 <= lower_bounds {
                continue;
            }

            let y = match view.calculate_y(price, self.logarithmic) {
                Some(y) => y,
                None => continue,
            };

            if y.is_infinite() || y.is_nan() {
                continue;
            }

            if y < upper_bounds {
                break;
            }

            // Draw grid line if grid is enabled
            if self.settings.object_settings.show {
                let grid_line = iced_graphics::geometry::Path::line(
                    Point::new(view.drawing_area.width, y),
                    Point::new(view.drawing_area.x, y),
                );
                frame.stroke(&grid_line, iced_graphics::geometry::Stroke {
                    width: self.settings.object_settings.size,
                    style: Style::from(color_from_template(&self.settings.object_settings.color)),
                    ..Default::default()
                });
            }

            if self.settings.text_settings.show && y < view.x_scale_area.y {
                let label = format!("{:.*}", self.decimal_precision, price);
                let text_point = Point::new(view.y_scale_area.x + 5.0, y - self.settings.text_settings.size / 2.0);
                if let Some(last_p_rectangle) = last_p_rectangle {
                    let buffer_space = self.settings.text_settings.size / 4.0; // Adjust the buffer space
                    if text_point.y >= last_p_rectangle.y - buffer_space && text_point.y <= last_p_rectangle.y + last_p_rectangle.height {
                        continue;
                    }
                }
                frame.fill_text(canvas::Text {
                    content: label,
                    position: text_point,
                    color: color_from_template(&self.settings.text_settings.color),
                    size: iced::Pixels(self.settings.text_settings.size),
                    ..Default::default()
                });
            }
        }

        // Draw the last price text
        if let Some(last_p_rectangle) = last_p_rectangle {
            let text_box = Path::rectangle(Point::new(last_p_rectangle.x, last_p_rectangle.y), Size::new(last_p_rectangle.width, last_p_rectangle.height));
            // Fill the text box with the object color
            frame.fill(&text_box, last_price_color);

            frame.fill_text(canvas::Text {
                content: last_price_text,
                position: Point::new(last_p_rectangle.x, last_p_rectangle.y),
                color:  color_from_template(&self.settings.text_settings.color),
                size: iced::Pixels(self.last_price_settings.text_settings.size),
                ..Default::default()
            });
        }

        if self.last_price_settings.object_settings.show {
            let last_price_y = (last_price_y - self.last_price_settings.text_settings.size / 5.0) + 2.0;
            let start_point = Point::new(view.drawing_area.x, last_price_y);
            let end_point = Point::new(view.drawing_area.x + view.drawing_area.width, last_price_y);

            let segments = vec![LASTPRICEDASHLENGTH, LASTPRICEGAPLENGTH];
            let line_dash = LineDash {
                segments: &segments,
                offset: 1,
            };
            let stroke = Stroke {
                style: last_price_color.into(),
                width: self.last_price_settings.object_settings.size,
                line_cap: Default::default(),
                line_join: Default::default(),
                line_dash,
            };
            let dash = Path::line(start_point, end_point);
            frame.stroke(
                &dash,
                stroke
            );
        }
    }

    fn calculate_grid_interval(&self, view: &ChartState) -> f64 {
        let price_range = view.y_high - view.y_low;
        // Estimate the number of intervals that can fit comfortably on the scale
        let target_intervals = view.y_scale_area.height / 200.0 ; // Adjusted for more space per label

        // Determine the raw interval value based on the price range and target number of intervals
        let raw_interval: f64 = price_range as f64 / target_intervals as f64  ;

        // Calculate the magnitude of the interval to round it to a sensible value
        let interval_magnitude = raw_interval.log10().floor();
        let interval_base = 10f64.powf(interval_magnitude);

        // Adjust interval to end in 0 or 5
        let mut adjusted_interval = if raw_interval / interval_base < 5.0 {
            interval_base * 0.5 // Target ending in 5
        } else {
            interval_base // Target ending in 0
        };

        // Adjust for very small or large intervals to ensure ending in 0 or 5
        if adjusted_interval < 1.0 {
            adjusted_interval = adjusted_interval * 2.0; // Ensure minimum interval respects the 5 ending
        }

        let scale = 10f64.powi(self.decimal_precision.clone() as i32);
        let rounded_interval = (adjusted_interval * scale).round() / scale;

        // Further adjust to ensure ending in 0 or 5
        if rounded_interval * scale % 10.0 != 0.0 && rounded_interval * scale % 5.0 != 0.0 {
            if (rounded_interval * scale % 10.0).abs() < 5.0 {
                return (rounded_interval * scale + (5.0 - rounded_interval * scale % 10.0)) / scale ;
            } else {
                return (rounded_interval * scale - (rounded_interval * scale % 10.0 - 5.0)) / scale ;
            }
        }

        rounded_interval
    }
}

fn adjust_to_nearest_five_or_zero(price: f64, grid_interval: f64) -> f64 {
    let remainder = price % grid_interval;
    if remainder != 0.0 {
        if remainder < grid_interval * 0.5 {
            return price - remainder;
        } else {
            return price + (grid_interval - remainder);
        }
    }
    price
}



