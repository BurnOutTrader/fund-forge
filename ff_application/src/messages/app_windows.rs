use iced::{Point, window};
use crate::application::windows::window_enum::WindowEnum;

/// Messages related to managing individual windows
#[derive(Debug, Clone)]
pub enum WindowMessage {
    ScaleInputChanged(window::Id, String),
    ScaleChanged(window::Id, String),
    TitleChanged(window::Id, String),
    CloseWindow(window::Id),
    WindowOpened(window::Id, Option<Point>),
    WindowClosed(window::Id),
    NewWindow(WindowEnum),
}
