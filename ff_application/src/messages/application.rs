use iced::Theme;
use crate::messages::app_windows::WindowMessage;
use crate::messages::panel_windows::PaneWindowMessage;

#[derive(Debug, Clone)]
pub enum ApplicationMessages {
    PaneWindowMessages(PaneWindowMessage),
    WindowMessages(WindowMessage),
    ThemeChanged(Theme),
    //ControlCenterUpdates(StrategyUpdates),
    Default
}

impl Default for ApplicationMessages {
    fn default() -> Self {
        ApplicationMessages::Default
    }
}
