use iced::mouse::ScrollDelta;
use crate::chart_canvas::graph::state::ChartState;

/// Zooms the price scale in or out by adjusting the GraphState.y_high and GraphState.y_low properties
pub fn zoom_price_range(state: &mut ChartState, delta: ScrollDelta) {

    if state.y_high == 0.0 || state.y_low == 0.0 {
        return;
    }

    // Normalize the scroll delta for mouse and trackpad and set the zoom direction
    let (zoom_in, mut y) = match delta {
        iced::mouse::ScrollDelta::Lines { y, .. } => (y < 0.0, y * 15.0),
        iced::mouse::ScrollDelta::Pixels { y, .. } => (y < 0.0, y),
    };

    y = y.abs();

    // Calculate the price range and the sensitivity factor
    let price_range = state.y_high - state.y_low;
    let sensitivity_factor = 0.05;
    let price_adjustment = price_range * sensitivity_factor;

    // Adjust the price range according to the zoom direction
    if zoom_in {
        state.y_high -= price_adjustment / 2.0;
        state.y_low += price_adjustment / 2.0;
    } else {
        state.y_high += price_adjustment / 2.0;
        state.y_low -= price_adjustment / 2.0;
    }
}

/// Zooms the date scale in or out by adjusting the GraphState.x_start and GraphState.x_end properties
pub fn zoom_or_scroll_date_range(state: &mut ChartState, delta: ScrollDelta) {
    let (scroll_right, mut x, mut y, zoom_in) = match delta {
        iced::mouse::ScrollDelta::Lines { x, y, .. } => (x < 0.0, x * 15.0, y * 15.0, y < 0.0),
        iced::mouse::ScrollDelta::Pixels { x, y, .. } => (x < 0.0, x, y, y < 0.0),
    };

    // Normalize 'x' and 'y' for consistency between input methods
    x = x.abs();
    y = y.abs();

    match y > x {
        true => zoom(state, zoom_in, x),
        false => pan(state, scroll_right, y),
    }
}

/// Zooms the chart by moving forward or backwards the GraphState.x_start property.
fn zoom(state: &mut ChartState, zoom_in: bool, x: f32) {
    let num_times = state.number_of_objects();// state.x_start - state.x_end / state.x_increment_factor as i64;
    if num_times == 0 {
        return;
    }
    // adjust scroll speed depending on the number of bars we have, more bars = faster scroll
    let sensitivity_factor = match zoom_in {
        true => match num_times {
            0..=20 => x.powf(0.0005),
            21..=50 => x.powf(0.005),
            51..=100 => x.powf(0.05),
            101..=200 => x.powf(1.0),
            _ => x.powf(1.5),
        },
        false => match num_times {
            0..=100 => x.powf(0.005),
            101..=200 => x.powf(0.0075),
            201..=300 => x.powf(0.01),
            _ => x.powf(1.0),
        },
    };

    let bars_to_shift = (x * sensitivity_factor).round();
    let duration = (state.x_increment_factor as f64 * bars_to_shift as f64) as i64;

    let earliest_time = state.x_start;

    //check if we want to zoom in or out
    match zoom_in {
        true => {
            state.x_start = earliest_time + duration;
        },
        false => {
            state.x_start = earliest_time - duration;
        }
    }

    if state.x_start >= state.x_end {
        state.x_start = state.x_end - state.x_increment_factor;
    }
}

/// Pans the chart by moving forward or backwards the `GraphState.date_range.earliest` and `GraphState.date_range.latest` properties.
/// Sliding our total state back or forward in time depending on a scroll direction without changing the number of bars shown.
/// It also allows us to scroll into the future if we are at the latest bar.
fn pan(state: &mut ChartState, scroll_right: bool, y: f32) {
    let sensitivity_factor = y.powf(0.2);

    let bars_to_shift = (y * sensitivity_factor).round() ;

    let duration = state.x_increment_factor * bars_to_shift as i64; //ToDO this was changed from duration so hopefully still works

    let earliest_time = state.x_start;
    let latest_time = state.x_end;

    //check if we want to pan right or left
    let( new_time_earliest, new_time_latest) = match scroll_right {
        true => (earliest_time + duration, latest_time + duration),
        false =>  (earliest_time - duration, latest_time - duration),
    };

    state.x_start = new_time_earliest;
    state.x_end = new_time_latest;
}