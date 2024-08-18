use std::fmt::Error;
use chrono_tz::Tz;
use iced::widget::canvas;
use iced::widget::canvas::Frame;
use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};
use ff_standard_lib::app::settings::GraphElementSettings;
use ff_standard_lib::helpers::converters::time_convert_utc_timestamp_to_fixed_offset;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use crate::canvas::graph::state::ChartState;
use crate::clicks::Click;
use crate::tools::lines::{HorizontalLine, VerticleLine};

//ToDo make drawing tool trait so we can implement depending on if strategy is using or if gui is using... or convert from strategy to gui type tool.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(
compare(PartialEq),
check_bytes,
)]
#[archive_attr(derive(Debug))]
pub enum DrawingTool {
    HorizontalLines(HorizontalLine),
    VerticleLines(VerticleLine),
}

impl DrawingTool {
    pub fn subscription(&self) -> &DataSubscription {
        match self {
            DrawingTool::HorizontalLines(object) => &object.subscription,
            DrawingTool::VerticleLines(object) => &object.subscription,
        }
    }

    pub fn from_array_bytes(data: &Vec<u8>) -> Result<Vec<DrawingTool>, Error> {
        let drawing_tools = match rkyv::check_archived_root::<Vec<DrawingTool>>(&data[..]){
            Ok(data) => data,
            Err(e) => {
                format!("Failed to deserialize tools: {}", e);
                return Err(Error);
            },
        };

        // Assuming you want to work with the archived data directly, or you can deserialize it further
        Ok(drawing_tools.deserialize(&mut rkyv::Infallible).unwrap())
    }

    pub fn is_ready(&self) -> bool {
        match self {
            DrawingTool::VerticleLines(object) => object.is_ready,
            DrawingTool::HorizontalLines(object) => object.is_ready,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            DrawingTool::HorizontalLines(_) => "H Line".to_string(),
            DrawingTool::VerticleLines(_) => "V Line".to_string(),
        }
    }
}

impl DrawingTool {
    pub fn draw(&self, time_zone: &Tz, view: &ChartState, frame: &mut Frame, cursor: iced::mouse::Cursor) {
        match self {
            DrawingTool::VerticleLines(verticle_line) => verticle_line.draw(time_zone, view, frame, cursor),
            DrawingTool::HorizontalLines(horizontal_line) => horizontal_line.draw(view, frame, cursor),
            // handle other variants...
        }
    }
    pub fn light_mode_color(&mut self) {
        match self {
            DrawingTool::VerticleLines(object) => object.settings = GraphElementSettings::light_mode_settings(),
            DrawingTool::HorizontalLines(object) => object.settings = GraphElementSettings::light_mode_settings(),
        }
    }

    pub fn dark_mode_color(&mut self) {
        match self {
            DrawingTool::VerticleLines(object) => object.settings = GraphElementSettings::dark_mode_settings(),
            DrawingTool::HorizontalLines(object) => object.settings = GraphElementSettings::dark_mode_settings(),
        }
    }

    pub fn id(&self) -> String {
        match self {
            DrawingTool::HorizontalLines(line) => line.id.clone(),
            DrawingTool::VerticleLines(line) =>  line.id.clone(),
        }
    }

    pub fn process_click(&self, view: &ChartState, click: Click) {
        /*match self.clone() {
        DrawingTool::VerticleLines(mut verticle_line) => verticle_line.process_click(view, click),
        DrawingTool::HorizontalLines(mut horizontal_line) => horizontal_line.process_click(view, click),
        // handle other variants...
    }*/
    }

    pub fn draw_tool(&self, time_zone: &Tz, view: &ChartState, frame: &mut canvas::Frame, cursor: iced::mouse::Cursor) {
        match self {
            DrawingTool::VerticleLines(verticle_line) => verticle_line.draw(time_zone, view, frame, cursor),
            DrawingTool::HorizontalLines(horizontal_line) => horizontal_line.draw(view, frame, cursor),
            // handle other variants...
        }
    }

    /// Converts the Tool time to the required chart offset
    pub fn x_alignment(utc_timestamp: i64, time_zone: &Tz) -> i64 {
        if time_zone == &Tz::UTC {
            utc_timestamp
        } else {
            time_convert_utc_timestamp_to_fixed_offset(time_zone, utc_timestamp, 0).timestamp()
        }
    }

    pub fn x_start(&self, time_zone: &Tz) -> Option<i64> {
        match self {
            DrawingTool::VerticleLines(object) => {
                if let Some(utc_time) = object.x_alignment {
                    Some(Self::x_alignment(utc_time, time_zone))
                } else {
                    None
                }
            }
            _ => None
        }
    }

    pub fn x_end(&self, time_zone: &Tz) -> Option<i64> {
        match self {
            DrawingTool::VerticleLines(object) => {
                if let Some(utc_time) = object.x_alignment {
                    Some(Self::x_alignment(utc_time, time_zone))
                } else {
                    None
                }
            }
            _ => None
        }
    }

    pub fn y_start(&self) -> Option<f64> {
        match self {
            DrawingTool::HorizontalLines(object) => {
                if let Some(price) = object.price {
                    Some(price)
                } else {
                    None
                }
            }
            _ => None
        }
    }

    pub fn y_end(&self) -> Option<f64> {
        match self {
            DrawingTool::HorizontalLines(object) => {
                if let Some(price) = object.price {
                    Some(price)
                } else {
                    None
                }
            }
            _ => None
        }
    }

    pub fn price(&self) -> Option<f64> {
        match self {
            DrawingTool::HorizontalLines(object) => {
                if let Some(price) = object.price {
                    Some(price)
                } else {
                    None
                }
            }
            _ => None
        }
    }

    pub fn lowest_price(&self) -> Option<f64> {
        match self {
            DrawingTool::HorizontalLines(object) => {
                if let Some(price) = object.price {
                    Some(price)
                } else {
                    None
                }
            }
            _ => None
        }
    }

    pub fn highest_price(&self) -> Option<f64> {
        match self {
            DrawingTool::HorizontalLines(object) => {
                if let Some(price) = object.price {
                    Some(price)
                } else {
                    None
                }
            }
            _ => None
        }
    }
}