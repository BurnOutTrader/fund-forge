use iced::widget::canvas;
use iced::mouse::Cursor;
use iced_graphics::geometry::{LineDash, Path, Stroke, Style};
use iced::Point;
use chrono::{DateTime};
use ff_standard_lib::gui_types::settings::{ColorTemplate, DisplaySettings, GraphElementSettings, TextSettings};
use iced_graphics::core::Color;
use crate::chart_canvas::graph::state::ChartState;

const TIME_LABEL_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const CROSSHAIR_DASH_LENGTH: f32 = 5.0;
const CROSSHAIR_GAP_LENGTH: f32 = 5.0;

pub fn color_from_template(color_template: &ColorTemplate) -> Color {
    Color::new(color_template.r, color_template.g, color_template.b, color_template.a)
}

#[derive(Debug, Clone)]
pub struct CrossHair {
    pub settings: GraphElementSettings,
    pub logarithmic: bool,
    pub decimal_precision: u8,
}

impl Default for CrossHair {
    fn default() -> Self {
        Self {
            settings: GraphElementSettings::dark_mode_settings(),
            logarithmic: false,
            decimal_precision: 5,
        }
    }
}

impl CrossHair {
    pub fn light_mode_settings(decimal_precision: u8, logarithmic: bool) -> Self {
        let text_settings = TextSettings {
            color: ColorTemplate::new(0.7, 0.7, 0.7, 1.0),
            size: 12.0,
            show: true,
        };
        let object_settings = DisplaySettings {
            color: ColorTemplate::new(0.7, 0.7, 0.7, 1.0),
            size: 1.0,
            show: true,
        };
        let settings = GraphElementSettings::new(object_settings, text_settings);
        Self {
            settings,
            logarithmic,
            decimal_precision,
        }
    }

    pub fn dark_mode_settings(decimal_precision: u8, logarithmic: bool) -> Self {
        let text_settings = TextSettings {
            color:  ColorTemplate::new(0.4, 0.4, 0.4, 1.0),
            size: 12.0,
            show: true,
        };
        let object_settings = DisplaySettings {
            color:  ColorTemplate::new(0.0, 0.0, 0.0, 1.0),
            size: 1.0,
            show: true,
        };
        let settings = GraphElementSettings::new(object_settings, text_settings);
        Self {
            settings,
            logarithmic,
            decimal_precision,
        }
    }

    pub fn draw(&self, view: &ChartState, frame: &mut canvas::Frame, cursor: Cursor) {
        let position = match cursor.position() {
            Some(position) => position,
            None => return,
        };

        if !view.drawing_area.contains(position) {
            return;
        }

        let segments = vec![CROSSHAIR_DASH_LENGTH, CROSSHAIR_GAP_LENGTH];
        let line_dash = LineDash {
            segments: &segments,
            offset: 1,
        };
        let stroke = Stroke {
            style: Style::from(color_from_template(&self.settings.object_settings.color)),
            width: self.settings.object_settings.size,
            line_cap: Default::default(),
            line_join: Default::default(),
            line_dash,
        };

        // Draw horizontal line
        let horizontal_line = Path::line(
            Point::new(view.drawing_area.x, position.y),
            Point::new(view.drawing_area.x + view.drawing_area.width, position.y),
        );
        frame.stroke(&horizontal_line, stroke.clone().with_color(color_from_template(&self.settings.object_settings.color)));

        // Calculate price and draw price label
        let price = view.value_at_y(position.y, self.logarithmic);
        let price_label = format!("{:.*}", self.decimal_precision as usize, price);
        frame.fill_text(canvas::Text {
            content: price_label,
            position: Point::new(view.drawing_area.x + view.drawing_area.width - (self.settings.text_settings.size * self.decimal_precision as f32), position.y),
            color: color_from_template(&self.settings.text_settings.color),
            size: iced::Pixels(self.settings.text_settings.size),
            ..Default::default()
        });

        if view.y_scale_area.contains(cursor.position().unwrap()) {
            return;
        }

        // let position_x = view.x_object_space_position(position.x); //neds to be centered in the object space
        // Draw vertical line at the locked x-coordinate
        let vertical_line = Path::line(
            Point::new(position.x, view.drawing_area.y),
            Point::new(position.x, view.drawing_area.y + view.drawing_area.height),
        );
        frame.stroke(&vertical_line, stroke.with_color(color_from_template(&self.settings.object_settings.color)));
        // Adjust the x-coordinate of the position

        let time = view.value_at_x(position.x);
        let time = DateTime::from_timestamp(time, 0).unwrap();
        let time_label = time.format(TIME_LABEL_FORMAT).to_string();

        // Calculate the width of the time label
        let time_label_width = time_label.len() as f32 * self.settings.text_settings.size / 2.0;

        let position_x = if position.x + time_label_width > view.drawing_area.x + view.drawing_area.width {
            position.x - time_label_width
        } else {
            position.x
        };

        frame.fill_text(canvas::Text {
            content: time_label,
            position: Point::new(position_x, view.drawing_area.y + view.drawing_area.height - self.settings.text_settings.size),
            color: color_from_template(&self.settings.text_settings.color),
            size: iced::Pixels(self.settings.text_settings.size),
            ..Default::default()
        });
    }
}
