use std::collections::BTreeMap;
use chrono::{Datelike, TimeZone, Utc};
use iced::{Point, Rectangle};
use iced::alignment::Horizontal;
use iced::widget::canvas::{Frame};
use iced_graphics::geometry::{Path, Text};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use ff_standard_lib::app::settings::GraphElementSettings;
use ff_standard_lib::standardized_types::enums::Resolution;
use crate::canvas::graph::state::ChartState;


/// A struct that holds the configuration for the display of a chart element.
/// This allows the grid for a `TimeScale` to be stored in advance and drawn when needed.
///
/// # Properties
/// * `importance`: The importance of the grid line. Used to determine which grid lines need to be prioritized.
/// * `time`: The time of the grid line.
/// * `time_string`: The string representation of the time.
/// * `x`: The x location of the grid line.
/// * `text`: The text object for the grid line.
/// * `grid_line`: The path for the grid line.
#[derive(Clone, Debug)]
struct TimeGrid {
    pub importance: DateScaleImportance,
    pub time: i64,
    pub time_string: String,
    pub x: f32,
    pub text: Text,
    pub grid_line: Path,
}

impl PartialEq for TimeGrid {
    fn eq(&self, other: &Self) -> bool {
        if self.time == other.time {
            return true
        }
        false
    }
}

/// A struct that holds the configuration for the display of a TimeScale for the X axis.
#[derive(Debug, Clone)]
pub struct TimeScale {
    pub settings: GraphElementSettings,
    pub resolution: Resolution,
}

impl Default for TimeScale {
    fn default() -> Self {
        TimeScale {
            settings: GraphElementSettings::light_mode_settings(),
            resolution: Resolution::default(),
        }
    }
}

impl TimeScale {
    pub fn light_mode_settings(resolution: Resolution) -> TimeScale {
        TimeScale {
            settings: GraphElementSettings::light_mode_settings(),
            resolution,
        }
    }

    pub fn dark_mode_settings(resolution: Resolution) -> TimeScale {
        TimeScale {
            settings: GraphElementSettings::dark_mode_settings(),
            resolution,
        }
    }

    pub fn scale_height(&self) -> f32 {
        let height = match self.settings.text_settings.show {
            true =>  self.settings.text_settings.size * 2.2,
            false => 0.0,
        };
        height
    }

    pub fn new(settings: GraphElementSettings, resolution: Resolution) -> TimeScale {
        let scale = TimeScale {
            settings,
            resolution,
        };
        scale
    }

    fn create_scale(&self, bounds: &Rectangle, scale_bounds: &Rectangle, view: &ChartState) -> BTreeMap<i64, TimeGrid> {
        let resolution_seconds = self.resolution.as_seconds();
        let mut grids: BTreeMap<i64, TimeGrid> = BTreeMap::new();

        let mut start_time = view.x_start;

        // setting this here ensures we always have a last_time
        let mut last_time = start_time - resolution_seconds;

        let end_time = view.x_end.clone();


        while start_time <= end_time {
            if grids.contains_key(&start_time) {
                start_time = start_time + resolution_seconds;
                last_time = start_time.clone();
                continue
            }

            let time_and_importance = rank_format_time(start_time.clone(), last_time, &self.resolution);

            // calculate the x (left) location of our date
            let true_x = match view.calculate_x(&start_time) {
                Some(x) => x,
                None => continue
            };

            let mut text = Text {
                content: time_and_importance.time_string.to_string(),
                position: Point::new(true_x, scale_bounds.center_y() - self.settings.text_settings.size / 2.0),
                color: self.settings.text_settings.color(),
                size: iced::Pixels(self.settings.text_settings.size),
                ..Default::default()
            };
            text.horizontal_alignment = Horizontal::Center;


            let grid_line = iced_graphics::geometry::Path::line(
                Point::new(true_x, scale_bounds.y),
                Point::new(true_x, scale_bounds.y - (bounds.height * (1.0 + view.percent_clear_boundaries))),
            );

            let grid = TimeGrid {
                importance: time_and_importance.importance,
                time: time_and_importance.time,
                time_string: time_and_importance.time_string,
                x: true_x,
                text,
                grid_line
            };

            grids.insert(grid.time, grid);
            last_time = start_time.clone();
            start_time = start_time + resolution_seconds;
        }
        grids
    }
}
///TODo think of frame as in frame rate, lets render each location of the time scale as a frame
impl TimeScale {
    pub fn draw_scale(&self, frame: &mut  Frame, view: &ChartState, bounds: &Rectangle) {
        let scale_bounds = view.x_scale_area;

        // Bind the borrow to a variable to extend its lifetime
        let scale_grids = self.create_scale(bounds, &scale_bounds, view);

        let mut grids_drawn: Vec<&TimeGrid> = Vec::new();
        let space_per_char = self.settings.text_settings.size / 1.5;
        let max_space_available = bounds.width;
        let mut space_taken = 0.0;

        for importance in DateScaleImportance::iter() {
            if space_taken >= max_space_available {
                break;
            }
            for (_, grid) in &scale_grids {
                if grid.importance != importance || space_taken >= max_space_available {
                    continue;
                }
                // Checking for overlap without cloning
                if !grids_drawn.is_empty() {
                    let overlap = grids_drawn.iter().any(|&existing_grid| {
                        (grid.x - existing_grid.x).abs() < (existing_grid.importance.chars() as f32 * space_per_char)
                    });
                    if overlap {
                        continue;
                    }
                }

                space_taken += grid.importance.chars() as f32 * space_per_char;

                // Draw the text for the date scale if show is enabled
                if self.settings.text_settings.show {
                    frame.fill_text(grid.text.clone()); // Consider optimizing text cloning if possible
                }

                if self.settings.object_settings.show {
                    frame.stroke(&grid.grid_line, iced_graphics::geometry::Stroke {
                        width: self.settings.object_settings.size,
                        style: self.settings.object_settings.color().into(),
                        ..Default::default()
                    });
                }

                if !grids_drawn.contains(&grid) {
                    grids_drawn.push(grid);
                }
            }
        }
    }
}

/// Ranks the importances of times in the date scale, new year being most important, new month 2nd, 3rd new day, last time of day.
#[derive(EnumIter, PartialEq, Clone, Debug)]
enum DateScaleImportance {
    Years,
    Months,
    Days,
    TimeOfDay
}
impl DateScaleImportance {
    /// Returns the average number of chars in the string for this DateScaleImportance
    pub fn chars(&self) -> i32 {
        match self {
            DateScaleImportance::Years => 4,
            DateScaleImportance::Months => 4,
            DateScaleImportance::Days => 5,
            DateScaleImportance::TimeOfDay => 8,
        }
    }
}

/// Groups a time with an importance value
#[derive(Clone, PartialEq, Debug)]
struct TimeAndImportance {
    time: i64,
    time_string: String,
    importance: DateScaleImportance,
}

/// Ranks the time with an importance enum and groups them by putting into a `TimeAndImportance` struct.
fn rank_format_time(time: i64, previous_time: i64, resolution: &Resolution) -> TimeAndImportance {
    let original_time = time.clone();
    let time = Utc.timestamp_opt(time, 0).unwrap();
    let previous_time = Utc.timestamp_opt(previous_time, 0).unwrap();

    let tuple_time_importance: (DateScaleImportance, &str) =
        if time.year() != previous_time.year() {
            (DateScaleImportance::Years, "%Y") // High importance, New year
        } else if time.month() != previous_time.month() {
            (DateScaleImportance::Months, "%b") // New month
        } else if time.day() != previous_time.day() {
            (DateScaleImportance::Days, "%e") // New day
        } else {
            match resolution {
                Resolution::Ticks(_) => (DateScaleImportance::TimeOfDay, "%H:%M:%S.f"), // Lowest importance intraday prices
                Resolution::Seconds(_) => (DateScaleImportance::TimeOfDay, "%H:%M:%S"),
                Resolution::Minutes(_) => (DateScaleImportance::TimeOfDay, "%H:%M"),
                Resolution::Hours(_) =>  (DateScaleImportance::TimeOfDay, "%H:%M") ,
            }
        };

    let time = time.format(tuple_time_importance.1).to_string();
    let importance = tuple_time_importance.0;
    TimeAndImportance {
        time: original_time,
        time_string: time,
        importance
    }
}