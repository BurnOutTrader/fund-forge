use iced::{Rectangle};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use ff_standard_lib::standardized_types::Price;

#[derive(Debug, PartialEq, Clone)]
pub struct ChartState {
    /// The start of the x visible series
    pub x_start:  i64,
    /// The end time of the x visible series
    pub x_end:  i64,
    /// The lowest value in the visible series
    pub y_low: f64,
    /// The highest value in the visible series
    pub y_high: f64,
    /// Used as a negative percentage to reduce the physical size of Geometry objects for each data object. `Example: size * (1.0 - space_percent)`
    pub space_percent: f32,
    /// The percentage of the drawing bounds to keep clear of objects
    pub percent_clear_boundaries: f32,
    /// The area of the bounds reserved for the x scale
    pub x_scale_area: Rectangle,
    /// The height of the x_scale to be reserved based on the scale settings.text_settings.size
    pub x_scale_height: f32,
    /// The area of the bounds reserved for the y scale
    pub y_scale_area: Rectangle,
    /// The width of the y_scale to be reserved based on the scale settings.text_settings.size multiplied by the length of the highest value
    pub y_scale_width: f32,
    /// The area of the bounds where it is possible to draw objects, it is possible to draw objects slightly higher and lower than the data area bounds.
    /// The displayed data is fit into a smaller window than the drawing area to allow easier drawing of objects at extreme high or low.
    pub drawing_area: Rectangle,
    /// The area of the bounds where the underlying data will be displayed.
    pub data_area: Rectangle,
    /// We can lock the view of the x scale to prevent auto-scaling
   pub x_locked: bool,
    /// We can lock the view of the y scale to prevent auto-scaling
    pub y_locked: bool,
    /// used to calculate moves forward in time
    pub last_x: i64,
    /// this is the multiplier for the x scale, for example to calulate the next x value =  last x value + x_increment_factor
    /// for time series this values is the resolution as seconds
    pub x_increment_factor: i64,

    pub is_initialized: bool,
}

impl ChartState {
        pub fn new(
            x_start: i64,
            x_end: i64,
            y_low: f64,
            y_high: f64,
            space_percent: f32,
            x_increment_factor: i64,
            percent_clear_boundaries: f32,
        ) -> Self {
            ChartState {
                x_start,
                x_end,
                y_low,
                y_high,
                space_percent,
                percent_clear_boundaries,
                x_scale_area: Rectangle::default(),
                y_scale_area: Rectangle::default(),
                data_area: Rectangle::default(),
                drawing_area: Rectangle::default(),
                x_scale_height: 0.0,
                y_scale_width:0.0,
                x_locked: false,
                y_locked: false,
                last_x: 0,
                x_increment_factor,
                is_initialized: false,
            }
        }

    pub fn initialize(&mut self, price_scale_height: f32, time_scale_width: f32, x_start: i64, x_end: i64, y_low: f64, y_high: f64, increment_factor: i64) {
        self.x_scale_height = time_scale_width;
        self.y_scale_width = price_scale_height;
        self.x_start = x_start;
        self.x_end = x_end.clone();
        self.y_low = y_low;
        self.y_high = y_high;
        self.x_increment_factor = increment_factor;
        self.x_locked = false;
        self.y_locked = false;
        self.last_x = x_end;
    }

    pub fn update_bounds(&mut self, bounds: &Rectangle, y_scale_width: f32, x_scale_height: f32) {
        let empty_space_top = bounds.height * self.percent_clear_boundaries / 2.0;
        let empty_space_bottom = bounds.height * self.percent_clear_boundaries / 2.0;

        // Adjust the data area to account for scales and clear boundaries
        let data_area = Rectangle {
            x: bounds.x,
            y: bounds.y + empty_space_top,
            width: bounds.width - y_scale_width,
            height: bounds.height - x_scale_height - empty_space_top - empty_space_bottom,
        };

        // Y scale is positioned to the right of the data area
        let y_scale_area = Rectangle {
            x: bounds.x + (bounds.width - y_scale_width),
            y: bounds.y,
            width: y_scale_width,
            height: bounds.height , // Match the height of the data area
        };

        // X scale is positioned below the data area
        let x_scale_area = Rectangle {
            x: data_area.x,
            y: bounds.y + (bounds.height - x_scale_height) ,
            width: data_area.width - self.y_scale_width,
            height: x_scale_height,
        };

        // Drawing area includes the data area plus potential drawing in the x and y scale areas
        let drawing_area = Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width - y_scale_width,
            height: bounds.height - x_scale_height,
        };

        self.data_area = data_area;
        self.drawing_area = drawing_area;
        self.y_scale_area = y_scale_area;
        self.x_scale_area = x_scale_area;
    }


    /// Uses the start and end time to calculate the width of the time section by dividing by resolution as seconds.
    /// # Returns
    /// `f32` the maximum width available per object if we are to fit all objects into the time window.
    pub fn x_object_space_width(&self) -> f32 {
        let num_objects = self.number_of_objects() as f64;
        if num_objects > 0.0 {
            (self.data_area.width as f64 / num_objects) as f32
        } else {
            0.0
        }
    }

    pub fn number_of_objects(&self) -> usize {
            let range_in_seconds = self.x_end - self.x_start;
            let resolution_in_seconds = self.x_increment_factor;
            if range_in_seconds == 0 || resolution_in_seconds == 0 {
                return 0;
            }
            let number_of_objects = range_in_seconds / resolution_in_seconds;
             number_of_objects as usize + 1
    }

    pub fn index_in_x(&self, time: &i64) -> Option<usize> {
        let total_range = (self.x_end - self.x_start) as f64;
        if total_range <= 0.0 {
            return None;
        }
        let position = (*time - self.x_start) as f64;
        let proportion = position / total_range;
        let index = (proportion * (self.number_of_objects() as f64 - 1.0)).round() as usize;
        Some(index)
    }

    pub fn calculate_x(&self, time: &i64) -> Option<f32> {
        let index = self.index_in_x(&time)?;
        let object_width = self.x_object_space_width();
        let x = self.data_area.x + (index as f32 * object_width) + (object_width / 2.0);
        Some(x)
    }

    pub fn value_at_x(&self, x: f32) -> i64 {
        let object_width = self.x_object_space_width();
        let relative_x = (x - self.data_area.x) / object_width;
        let index = relative_x.round() as usize;
        let total_range = (self.x_end - self.x_start) as f64;
        let position = index as f64 / (self.number_of_objects() as f64 - 1.0);
        let time = self.x_start + (position * total_range).round() as i64;
        time
    }

    /// Calculates the y-coordinate of the price
    pub fn calculate_y(&self, value: Price, logarithmic: bool) -> Option<f32> {
        let adjusted_height = self.data_area.height as f64 * (1.0 - self.percent_clear_boundaries as f64);

        if logarithmic {
            if value <= dec!(0.0) || self.y_low <= 0.0 || self.y_high <= 0.0 {
                return None; // Guard against invalid input
            }
            let log_low = self.y_low.ln();
            let log_high = self.y_high.ln();
            let log_value = value.to_f64().unwrap().ln();
            let proportion = (log_value - log_low) / (log_high - log_low);
            let y = self.data_area.y as f64 + self.data_area.height as f64 * self.percent_clear_boundaries as f64 / 2.0 + adjusted_height - (proportion * adjusted_height);
            Some(y as f32)
        } else {
            // For linear scale
            if self.y_high == self.y_low { // Guard against division by zero
                return None;
            }
            let proportion = (value.to_f64().unwrap() - self.y_low) / (self.y_high - self.y_low);
            let y = self.data_area.y as f64 + self.data_area.height as f64 * self.percent_clear_boundaries as f64 / 2.0 + adjusted_height - (proportion * adjusted_height);
            Some(y as f32)
        }
    }

    pub fn value_at_y(&self, y: f32, logarithmic: bool) -> f64 {
        let data_area = self.data_area;
        let adjusted_height = data_area.height as f64 * (1.0 - self.percent_clear_boundaries as f64);
        let relative_y = (y as f64 - (data_area.y as f64 + data_area.height as f64 * self.percent_clear_boundaries as f64 / 2.0)) / adjusted_height;
        let proportion = 1.0 - relative_y; // Invert because y increases downwards

        if logarithmic {
            if self.y_low <= 0.0 || self.y_high <= 0.0 || self.y_high == self.y_low {
                return 0.0; // Logarithmic scale doesn't support non-positive values or zero range
            }
            let log_low = self.y_low.ln();
            let log_high = self.y_high.ln();
            let value_log = proportion * (log_high - log_low) + log_low;
            value_log.exp()
        } else {
            if self.y_high == self.y_low {
                return self.y_low; // Avoid division by zero for linear scale with zero range
            }
            let total_value_range = self.y_high - self.y_low;
            let value_difference = proportion * total_value_range;
            self.y_low + value_difference
        }
    }
}


impl Default for ChartState {
    fn default() -> Self {
        ChartState {
            x_start: 0,
            x_end: 0,
            y_low: 0.0,
            y_high: 0.0,
            space_percent: 0.2,
            percent_clear_boundaries: 0.05,
            x_scale_area:Rectangle::default(),
            y_scale_area: Rectangle::default(),
            data_area: Rectangle::default(),
            drawing_area: Rectangle::default(),
            x_scale_height: 0.0,
            y_scale_width: 0.0,
            x_locked: false,
            y_locked: false,
            last_x: 0,
            x_increment_factor: 0,
            is_initialized: false,
        }
    }
}
