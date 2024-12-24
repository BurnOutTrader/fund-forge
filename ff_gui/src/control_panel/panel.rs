use iced::{window, Alignment, Element, Length, Size, Theme};
use ff_standard_lib::standardized_types::accounts::Account;
use ff_standard_lib::strategies::strategy_events::{StrategyControls, StrategyEvent};
use iced::widget::{button, container, row, svg, text};
use tokio::sync::mpsc;

pub fn window_settings(account: &Account) -> window::Settings {
    window::Settings {
        size: Size::new(400.0, 250.0),
        position: Default::default(),
        min_size: None,
        max_size: None,
        visible: true,
        resizable: true,
        decorations: true,
        transparent: false,
        level: Default::default(),
        icon: None,
        platform_specific: Default::default(),
        exit_on_close_request: false,
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ContinuePressed,
    PausePressed,
    StopPressed,
    StartPressed,
    ReducePositionSize,
    IncreasePositionSize,
    Flatten
}

pub struct StrategyControlPanel {
    pub current_state: StrategyControls,
    pub strategy_sender: mpsc::Sender<StrategyEvent>,
    pub theme: Theme,
}

fn new_strategy_control(strategy_sender: mpsc::Sender<StrategyEvent>, theme: Theme) -> StrategyControlPanel {
    StrategyControlPanel {
        strategy_sender,
        current_state: StrategyControls::Continue,
        theme,
    }
}

impl StrategyControlPanel {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ContinuePressed => {
                if let Ok(_) = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Continue)) {
                    self.current_state = StrategyControls::Continue;
                }
            }
            Message::PausePressed => {
                if let Ok(_) = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Pause)) {
                    self.current_state = StrategyControls::Pause;
                }
            }
            Message::StopPressed => {
                if let Ok(_) = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Stop)) {
                    self.current_state = StrategyControls::Stop;
                }
            }
            Message::StartPressed => {
                if let Ok(_) = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Start)) {
                    self.current_state = StrategyControls::Start;
                }
            }
            Message::ReducePositionSize => {
                let _ = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Custom("Reduce".to_string())));
            }
            Message::IncreasePositionSize => {
                let _ = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Custom("Increase".to_string())));
            },
            Message::Flatten => {
                let _ = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Custom("Flatten".to_string())));
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let icon_size = 20;

        let control_buttons = row![
        button(
            row![
                svg(svg::Handle::from_memory(PLAY_ICON.as_bytes()))
                    .width(Length::Fixed(icon_size as f32))
                    .height(Length::Fixed(icon_size as f32)),
            ].spacing(10).align_y(Alignment::Center)
        )
            .on_press(Message::ContinuePressed)
            .padding(10),
        button(
            row![
                svg(svg::Handle::from_memory(PAUSE_ICON.as_bytes()))
                    .width(Length::Fixed(icon_size as f32))
                    .height(Length::Fixed(icon_size as f32)),
            ].spacing(10).align_y(Alignment::Center)
        )
            .on_press(Message::PausePressed)
            .padding(10),
        button(
            row![
                svg(svg::Handle::from_memory(STOP_ICON.as_bytes()))
                    .width(Length::Fixed(icon_size as f32))
                    .height(Length::Fixed(icon_size as f32)),
            ].spacing(10).align_y(Alignment::Center)
        )
            .on_press(Message::StopPressed)
            .padding(10),
        button(
            row![
                svg(svg::Handle::from_memory(POWER_ICON.as_bytes()))
                    .width(Length::Fixed(icon_size as f32))
                    .height(Length::Fixed(icon_size as f32)),
            ].spacing(10).align_y(Alignment::Center)
        )
            .on_press(Message::StartPressed)
            .padding(10),
        button(
            row![
                svg(svg::Handle::from_memory(REDUCE_ICON.as_bytes()))
                    .width(Length::Fixed(icon_size as f32))
                    .height(Length::Fixed(icon_size as f32)),
            ].spacing(10).align_y(Alignment::Center)
        )
            .on_press(Message::ReducePositionSize)
            .padding(10),

            button(
            row![
                svg(svg::Handle::from_memory(INCREASE_ICON.as_bytes()))
                    .width(Length::Fixed(icon_size as f32))
                    .height(Length::Fixed(icon_size as f32)),
            ].spacing(10).align_y(Alignment::Center)
        )
            .on_press(Message::IncreasePositionSize)
            .padding(10),

            button(
            row![
                svg(svg::Handle::from_memory(FLATTEN_ICON.as_bytes()))
                    .width(Length::Fixed(icon_size as f32))
                    .height(Length::Fixed(icon_size as f32)),
            ].spacing(10).align_y(Alignment::Center)
        )
            .on_press(Message::Flatten)
            .padding(10),
    ]
            .spacing(10)
            .align_y(Alignment::Center);

        let message = match self.current_state {
            StrategyControls::Continue => "Running".to_string(),
            StrategyControls::Pause => "Paused".to_string(),
            StrategyControls::Stop => "Stopped".to_string(),
            StrategyControls::Start => "Running".to_string(),
            _ => "".to_string()
        };

        let status = text(message)
            .size(20);

        let content = iced::widget::column![
        control_buttons,
        status,
    ]
            .spacing(20)
            .align_x(Alignment::Center);

        container(content)
            .width(Length::Fixed(400.0))
            .height(Length::Fixed(250.0))
            .into()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

const PLAY_ICON: &str = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor">
    <polygon points="5 3 19 12 5 21 5 3"/>
</svg>"#;

const PAUSE_ICON: &str = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor">
    <rect x="6" y="4" width="4" height="16"/>
    <rect x="14" y="4" width="4" height="16"/>
</svg>"#;

const STOP_ICON: &str = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor">
    <rect x="4" y="4" width="16" height="16"/>
</svg>"#;

const POWER_ICON: &str = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor">
    <path d="M18.36 6.64a9 9 0 1 1-12.73 0"/>
    <line x1="12" y1="2" x2="12" y2="12"/>
</svg>"#;

const REDUCE_ICON: &str = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10" />
    <line x1="8" y1="12" x2="16" y2="12" />
</svg>"#;

const INCREASE_ICON: &str = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="8" x2="12" y2="16" />
    <line x1="8" y1="12" x2="16" y2="12" />
</svg>"#;

const FLATTEN_ICON: &str = r#"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
    <line x1="3" y1="3" x2="21" y2="21" />
</svg>"#;