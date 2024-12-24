use std::collections::BTreeMap;
use std::hash::Hash;
use iced::{Application, Color, Rectangle, Renderer, Theme};
use iced::event::Status;
use iced::mouse::{Cursor, Interaction};
use iced::widget::canvas;
use iced::widget::canvas::{Event, Frame, Geometry};
use chrono_tz::Tz;
use ff_standard_lib::messages::registry_messages::guis::RegistryGuiResponse;
use iced::advanced::Widget;
use iced::advanced::subscription::Recipe;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use crate::chart_canvas::graph::enums::areas::ChartAreas;
use crate::chart_canvas::graph::enums::x_scale::XScale;
use crate::chart_canvas::graph::enums::y_scale::YScale;
use crate::chart_canvas::graph::models::crosshair::CrossHair;
use crate::chart_canvas::graph::models::data::SeriesData;
use crate::chart_canvas::graph::state::ChartState;
use crate::chart_canvas::graph::view;
use crate::clicks::click_location;
use ff_standard_lib::standardized_types::base_data::base_data_enum::BaseDataEnum;
use ff_standard_lib::standardized_types::base_data::traits::BaseData;

///  A graph is the canvases object responsible for bringing the other elements that make up a graph, it is responsible for drawing, update and event state.
/// The state property manages the sections of the graph, scale areas etc and converts position.x and positiion.y into values according to the graph data set.
#[derive(Debug, Clone)]
pub struct ChartCanvas {
    pub data: BTreeMap<i64, Vec<SeriesData>>,
    pub background_color: Color,
    pub x_scale: XScale,
    pub bounds: Rectangle,
    pub y_scale: YScale,
    pub crosshair: CrossHair,
    pub time_zone: Tz,
    pub data_added: Option<i64>, //todo... THIS WAS REMOVED FROM UPDATE ETC if data is added we hold the x_value of the data here so that we can update our state in update().
}

impl ChartCanvas {
    pub fn new(data: BTreeMap<i64, Vec<SeriesData>>, background_color: Color, x_scale: XScale, bounds: Rectangle, y_scale: YScale, crosshair: CrossHair, time_zone: Tz) -> Self {
        ChartCanvas {
            data,
            background_color,
            x_scale,
            bounds,
            y_scale,
            crosshair,
            time_zone,
            data_added: None,
        }
    }

    pub fn return_range(&self, from: i64, to: i64) -> BTreeMap<i64, Vec<SeriesData>> {
        if from > to {
            return BTreeMap::new();
        }
        let mut data = BTreeMap::new();

        for (&key, &ref value) in self.data.range(from..=to) {
            data.insert(key.clone(), value.clone());
        }

        data
    }

    pub fn update(&mut self, base_data: &BaseDataEnum) {
        //println!("{}", base_data);
        self.data_added = None;
        let time_zone = self.time_zone.clone();

        let time = base_data.time_local(&time_zone).timestamp();
        //if the data is bar type this will update the last price, if not then it will have no effect
        let mut last_open_price: Option<Decimal> = None;
        let mut last_price: Option<Decimal> = None;

        // Insert the new data
        if !self.data.contains_key(&time) {
            self.data.insert(time.clone(), Vec::new());
        }

        // Loop through our new data by type and make any type specific updates

        let mut data_added = false;
        match base_data {
            BaseDataEnum::Candle(candle_stick) => {
                last_open_price = Some(candle_stick.open);
                last_price = Some(candle_stick.close);
                if let Some(last_open_price) = last_open_price {
                    self.y_scale.last_comparison_value(last_open_price)
                }
                if let Some(last_price) = last_price {
                    self.y_scale.last_value(last_price)
                }

                if let Some(vec) = self.data.get_mut(&time) {
                    // Collect the indices of elements to remove
                    let mut indices_to_remove = Vec::new();
                    let mut data_added = false;

                    for (i, data) in vec.iter_mut().enumerate() {
                        if data.time_utc() == candle_stick.time_utc().timestamp() {
                            indices_to_remove.push(i);
                        }
                    }

                    // Remove elements in reverse order to avoid shifting indices
                    for &i in indices_to_remove.iter().rev() {
                        vec.remove(i);
                    }

                    // Add the new candle stick data
                    vec.push(SeriesData::CandleStick(candle_stick.clone()));
                    data_added = true;
                } else {
                    // Insert a new vector if the time key does not exist
                    self.data.insert(
                        candle_stick.time_utc().timestamp(),
                        vec![SeriesData::CandleStick(candle_stick.clone())],
                    );
                    data_added = true;
                }
            },
            _ => {}
        }

        if data_added {
            self.data_added = Some(time);
        }
    }


    pub fn autoscale_y(&self, state: &mut ChartState, data_range: &BTreeMap<i64, Vec<SeriesData>>) {
        if state.y_locked {
            return;
        }

        let mut y_high = Decimal::MIN;
        let mut y_low = Decimal::MAX;

        if data_range.is_empty() {
            return;
        }

        for (_, data_vec) in data_range {
            if let Some(max_data) = data_vec.iter().max_by(|x, y| x.highest_value().partial_cmp(&y.highest_value()).unwrap()) {
                y_high = y_high.max(max_data.highest_value());
            }
            if let Some(min_data) = data_vec.iter().min_by(|x, y| x.lowest_value().partial_cmp(&y.lowest_value()).unwrap()) {
                y_low = y_low.min(min_data.lowest_value());
            }
        }

        if y_high != Decimal::MIN {
            state.y_high = y_high.to_f64().unwrap();
        }
        if y_low != Decimal::MAX {
            state.y_low = y_low.to_f64().unwrap();
        }
    }

    pub fn autoscale_x(&self, state: &mut ChartState) {
        if state.x_locked {
            return;
        }

        // Calculate the duration to be shown based on the current state
        let duration_multiple = state.x_end - state.x_start;
        if duration_multiple == 0 {
            // If the duration is 0, there's nothing to adjust
            return;
        }

        // Find the latest timestamp in the data
        let end = self.data.keys().last().cloned().unwrap_or_else(|| state.x_end);

        // Adjust state.x_start to be 'duration_multiple' seconds before 'end'
        // This ensures that the state shows a fixed duration ending with the latest data point
        state.x_start = end - duration_multiple;
        state.x_end = end; // Ensure state.x_end is aligned with the latest data point
    }

    fn double_left_click(&self, state: &mut ChartState, cursor: Cursor) {
        let click_location = click_location(state, cursor.position().unwrap());
        match click_location {
            ChartAreas::PriceScale => {
                state.y_locked = false;
                let data_range = self.return_range(state.x_start, state.x_end);
                self.autoscale_y(state, &data_range);
            },
            ChartAreas::DateScale => {
                state.x_locked = false;
                state.y_locked = false;
                self.autoscale_x(state);
                let data_range = self.return_range(state.x_start, state.x_end);
                self.autoscale_y(state, &data_range);
            },
            _ => {}
        }
    }
}


impl canvas::Program<RegistryGuiResponse> for ChartCanvas {
    type State = ChartState;

    fn update(&self, state: &mut Self::State, event: Event, bounds: Rectangle, cursor: Cursor) -> (Status, Option<RegistryGuiResponse>) {
        if bounds.width == 0.0 || bounds.height == 0.0 {
            return (Status::Ignored, None);
        }

        if !state.is_initialized {
            let x_start = self.data.keys().next().cloned().unwrap_or_else(|| 0);
            let x_end = self.data.keys().last().cloned().unwrap_or_else(|| 0);
            let y_low = get_lowest_low(&self.data);
            let y_high = get_highest_high(&self.data);
            let increment_factor = self.x_scale.increment_factor();
            state.initialize(self.y_scale.scale_width(), self.x_scale.scale_height(),  x_start, x_end, y_low.to_f64().unwrap(), y_high.to_f64().unwrap(), increment_factor);
            state.is_initialized = true;
        }

        if self.bounds != bounds {
            state.update_bounds(&bounds, self.y_scale.scale_width(), self.x_scale.scale_height());
        }

        if let Some(time) = self.data_added {
            if !state.x_locked {
                let time_difference = time - state.last_x;
                // Update the state window
                if state.x_start == 0 {
                    state.x_start = time - (self.x_scale.increment_factor() * 250);
                    state.x_end = time;
                } else {
                    state.x_start += time_difference;
                    state.x_end = time;
                }
                
            }
            state.last_x = time;

            let data_range = self.return_range(state.x_start, state.x_end);
            self.autoscale_y(state, &data_range);
        }


        match event {
            Event::Mouse(mouse_event) => {
                match mouse_event {
                    iced::mouse::Event::WheelScrolled { delta } => {
                        if let Some(position) = cursor.position() {
                           if state.drawing_area.contains(position) || state.x_scale_area.contains(position) {
                               state.y_locked = false;
                               state.x_locked = false;
                               view::zoom_or_scroll_date_range(state, delta);
                               self.autoscale_y(state, &self.return_range(state.x_start, state.x_end));
                                state.x_locked = true;
                            } else if state.y_scale_area.contains(position) {
                                state.y_locked = false;
                               view::zoom_price_range(state, delta);
                                state.y_locked = true;
                            }
                        }
                    },
                    iced::mouse::Event::ButtonReleased(button) => {
                        if bounds.contains(cursor.position().unwrap()) {
                            self.double_left_click(state, cursor);
                        }
                    },
                    _ => {}
                }
            }
            _ => {}
        }
        (Status::Captured, None)
    }

    fn draw(&self, state: &Self::State, renderer: &Renderer, theme: &Theme, bounds: Rectangle, cursor: Cursor) -> Vec<Geometry> {
        // Create a vector to hold the geometries
        let mut frame = Frame::new(renderer, bounds.size());

        // Create and fill the background path
        let background_path = iced::widget::canvas::Path::rectangle(iced::Point::new(0.0, 0.0), bounds.size());
        frame.fill(&background_path, self.background_color);


        if !self.data.is_empty() {
            let range_data = self.return_range(state.x_start, state.x_end);
            self.autoscale_y(&mut state.clone(), &range_data);
            if !self.data.is_empty() {
                let range_data = self.return_range(state.x_start, state.x_end);
                self.autoscale_y(&mut state.clone(), &range_data);
                SeriesData::draw_data(&mut frame, &state, &bounds, &range_data, self.y_scale.logorithmic(), &self.time_zone);
            }
        }

        self.x_scale.draw(&mut frame, &state, &bounds);

        self.y_scale.draw(&mut frame, &state, &bounds);

        self.crosshair.draw(&state, &mut frame, cursor);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Interaction {
        match cursor {
            /*Cursor::Available(point) => {
                if !bounds.contains(cursor.position().unwrap()) {
                    return Interaction::default();
                }
               if  !state.x_scale_area.contains(point) && !state.y_scale_area.contains(point) {
                    Interaction::Crosshair
                } else if state.y_scale_area.contains(point) {
                    Interaction::ResizingVertically
                } else if state.x_scale_area.contains(point) {
                    Interaction::ResizingHorizontally
                } else {
                    Interaction::Idle
                }
            }*/
            _ => Interaction::default(),
        }
    }
}


pub fn get_highest_high(data: &BTreeMap<i64, Vec<SeriesData>>) -> Decimal {
    let mut high = dec!(0.0);
    for (_, data) in data.iter() {
        for data in data.iter() {
            let data_high = data.highest_value();
            if data_high > high {
                high = data_high
            }
        }
    }
    high
}

pub fn get_lowest_low(data: &BTreeMap<i64, Vec<SeriesData>>) -> Decimal {
    let mut low = Decimal::MAX;
    for (_, data) in data.iter() {
        for data in data.iter() {
            let data_low = data.lowest_value();
            if data_low < low {
                low = data_low;
            }
        }
    }
    low
}



