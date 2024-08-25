use chrono_tz::Tz;
use iced::Point;
use iced::widget::canvas;
use iced::Color;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use crate::app::settings::GraphElementSettings;
use crate::standardized_types::subscriptions::DataSubscription;
use crate::drawing_objects::drawing_tool_enum::DrawingTool;

/// A struct that represents a vertical line on the graph.
///
/// # Fields
///
/// * `time_utc`: The time value of the vertical line. charts need to manually convert to other times zones.
/// * `offset`: The time zone offset of the vertical line.
/// * `settings`: The settings of the vertical line.
/// * `x_increment_factor`: The factor used to increment the x-axis value for the vertical line. for time series this values is the resolution as seconds.
/// * `id`: The unique ID of the vertical line.
/// * `is_ready`: A boolean indicating whether the vertical line is ready to be drawn.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct VerticleLine {
    pub x_alignment: Option<i64>,
    pub settings: GraphElementSettings,
    x_increment_factor: i64,
    pub id: String,
    pub is_ready: bool,
    pub subscription: DataSubscription,
}

impl VerticleLine {

    pub fn new_time_series(id:String, settings: GraphElementSettings, subscription: DataSubscription, is_ready: bool) -> Self {
        Self {
            x_alignment: None,
            settings,
            x_increment_factor: subscription.resolution.as_seconds(),
            id,
            is_ready,
            subscription,
        }
    }

    pub fn new(id:String, settings: GraphElementSettings, subscription: DataSubscription, x_increment_factor: i64, is_ready: bool) -> Self {
        Self {
            x_alignment: None,
            settings,
            x_increment_factor,
            id,
            is_ready,
            subscription,
        }
    }
    
    pub fn id(&self) -> &str {
        &self.id
    }
    
    /*pub fn draw(&self, time_zone: &Tz, state: &ChartState, frame: &mut canvas::Frame, cursor: iced::mouse::Cursor) {
        if let Some(utc_time) = self.x_alignment {
            let time = DrawingTool::x_alignment(utc_time, time_zone);
            let position = match self.is_ready {
                false => if let Some(cursor_position) = cursor.position() {
                    cursor_position
                } else {
                    return;
                },
                true => {
                    let x = match state.calculate_x(&time) {
                        Some(x) => x,
                        None => return,
                    };
                    let y = state.drawing_area.y;
                    Point::new(x, y)
                }
            };
            if !state.drawing_area.contains(position) {
                return;
            }

            let color: Color = match self.is_ready {
                false => {
                    self.settings.object_settings.fade_color()
                },
                true => self.settings.object_settings.color()
            };

            let width = self.settings.object_settings.size;
            let height = state.drawing_area.height;
            let line = canvas::Path::line(Point::new(position.x, state.drawing_area.y), Point::new(position.x, state.drawing_area.y + height));
            frame.stroke(&line, Stroke::default().with_color(color).with_width(width));
        }
    }*/
}

/// A struct that represents a horizontal line on the graph.
/// # Fields
/// * `price`: The price value of the horizontal line.
/// * `offset`: The time zone offset of the horizontal line.
/// * `logarithmic`: A boolean indicating whether the horizontal line is logarithmic.
/// * `settings`: The settings of the horizontal line.
/// * `x_increment_factor`: The factor used to increment the x-axis value for the horizontal line. for time series this values is the resolution as seconds.
/// * `id`: The unique ID of the horizontal line.
/// * `is_ready`: A boolean indicating whether the horizontal line is ready to be drawn.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub struct HorizontalLine {
    pub price: Option<f64>,
    logarithmic: bool,
    pub settings: GraphElementSettings,
    pub x_increment_factor: i64, // this is resolution.to_seconds() for time series
    pub id: String,
    pub is_ready: bool,
    pub subscription: DataSubscription,
}

impl HorizontalLine {

    pub fn new_time_series(id: String, settings: GraphElementSettings, subscription: DataSubscription, logarithmic: bool, is_ready: bool) -> Self {
        Self {
            price : None,
            settings,
            x_increment_factor: subscription.resolution.as_seconds(),
            id,
            logarithmic,
            is_ready,
            subscription
        }
    }

    pub fn new(id: String, settings: GraphElementSettings, x_increment_factor:  i64, subscription: DataSubscription, logarithmic: bool, is_ready: bool) -> Self {
        Self {
            price : None,
            settings,
            x_increment_factor,
            id,
            is_ready,
            logarithmic,
            subscription
        }
    }
    
    pub fn id(&self) -> &str {
        &self.id
    }

  /*  pub fn process_click(&mut self, view: &ChartState, click: Click) {
        let price = view.value_at_y(click.position.y, self.logarithmic);
        self.is_ready = true;
        self.update_price(price);
    }*/

    pub fn update_price(&mut self, price: f64) {
        self.price = Some(price);
    }

   /* pub fn draw(&self, view: &ChartState, frame: &mut canvas::Frame, cursor: iced::mouse::Cursor) {
        let position = match self.is_ready {
            false => if let Some(cursor_position) = cursor.position() {
                cursor_position
            } else {
                return;
            },
            true => {
                let x = view.drawing_area.x;
                let y = match view.calculate_y(self.price.unwrap(), self.logarithmic) {
                    Some(y) => y,
                    None => { return; }
                };
                Point::new( x, y)
            }
        };
        if !view.drawing_area.contains(position) {
            return;
        }

        let color = match self.is_ready {
            false =>  self.settings.object_settings.fade_color(),
            true => self.settings.object_settings.color()
        };

        let width = view.drawing_area.width ;
        let line = canvas::Path::line(Point::new(position.x, view.drawing_area.x), Point::new(view.drawing_area.x, position.y + width));
        frame.stroke(&line, Stroke::default().with_color(color).with_width( self.settings.object_settings.size));

    }*/
}
