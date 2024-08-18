use iced::{Element, Theme, window};
use iced::widget::text_input::Id;
use crate::application::traits::WindowTrait;
use crate::application::windows::strategy::window::StrategyWindow;
use crate::messages::application::ApplicationMessages;

#[derive(Debug, Clone)]
pub enum WindowEnum {
    Strategy(StrategyWindow),
}

impl WindowEnum {
    pub fn view(&self, id: window::Id) -> Element<ApplicationMessages> {
        match self {
            WindowEnum::Strategy(window) => window.view(id),
        }
    }
}

impl WindowTrait for WindowEnum {
    fn title(&self) -> String {
        match self {
            WindowEnum::Strategy(w) => w.title(),
        }
    }

    fn scale_input(&self) -> &str {
        match self {
            WindowEnum::Strategy(w) => w.scale_input(),
        }
    }

    fn current_scale(&self) -> f64 {
        match self {
            WindowEnum::Strategy(w) => w.current_scale(),
        }
    }

    fn theme(&self) -> &Theme {
        match self {
            WindowEnum::Strategy(w) => w.theme(),
        }
    }

    fn input_id(&self) -> Id {
        match self {
            WindowEnum::Strategy(w) => w.input_id(),
        }
    }

    fn set_title(&mut self, title: String) {
        match self {
            WindowEnum::Strategy(w) => w.set_title(title),
        }
    }

    fn set_scale_input(&mut self, scale_input: String) {
        match self {
            WindowEnum::Strategy(w) => w.set_scale_input(scale_input),
        }
    }

    fn set_current_scale(&mut self, current_scale: f64) {
        match self {
            WindowEnum::Strategy(w) => w.set_current_scale(current_scale),
        }
    }

    fn set_theme(&mut self, theme: Theme) {
        match self {
            WindowEnum::Strategy(w) => w.set_theme(theme),
        }
    }
}


