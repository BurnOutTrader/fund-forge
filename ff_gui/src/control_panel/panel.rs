use iced::{window, Alignment, Element, Length, Size, Theme};
use iced::advanced::widget::Text;
use ff_standard_lib::standardized_types::accounts::Account;
use ff_standard_lib::strategies::strategy_events::{StrategyControls, StrategyEvent};
use iced::widget::{button, container, row, svg, text, Column, Radio, Row, Slider};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use ff_standard_lib::standardized_types::enums::Bias;

pub fn window_settings() -> window::Settings {
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
    Flatten,
    RiskReward(f64),
    Bias(Bias),
}

pub struct StrategyControlPanel {
    pub current_state: StrategyControls,
    pub strategy_sender: mpsc::Sender<StrategyEvent>,
    pub theme: Theme,
    pub risk_reward: f64,
    pub bias: Bias,
}

pub fn new_strategy_control(strategy_sender: mpsc::Sender<StrategyEvent>, theme: Theme, risk_reward: Decimal, bias: Bias) -> StrategyControlPanel {
    StrategyControlPanel {
        strategy_sender,
        current_state: StrategyControls::Continue,
        theme,
        risk_reward: risk_reward.to_f64().unwrap(),
        bias
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
            Message::RiskReward(risk_reward) => {
                self.risk_reward = risk_reward;
                let _ = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Custom(format!("Risk Reward:{}", risk_reward))));
            }
            Message::Bias(bias) => {
                self.bias = bias;
                match bias {
                    Bias::Bullish => {
                        let _ = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Custom("Trade Long".to_string())));
                    }
                    Bias::Bearish => {
                        let _ = self.strategy_sender.try_send(StrategyEvent::StrategyControls(StrategyControls::Custom("Trade Short".to_string())));
                    }
                    Bias::Neutral => {}
                }
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

        let risk_reward_slider = Column::new()
            .push(Text::new("Risk/Reward Ratio").size(16))
            .push(
                Slider::new(
                    1.0..=10.0,
                    self.risk_reward,
                    Message::RiskReward,
                )
                    .step(0.1)
                    .width(Length::Fixed(200.0))
            )
            .push(Text::new(format!("{:.1}", self.risk_reward)).size(14))
            .spacing(10)
            .align_x(Alignment::Center);

        // Bias radio buttons
        let bias_controls = Column::new()
            .push(Text::new("Trading Bias").size(16))
            .push(
                Row::new()
                    .push(Radio::new(
                        "Bullish",
                        Bias::Bullish,
                        Some(self.bias.clone()),
                        Message::Bias,
                    ))
                    .push(Radio::new(
                        "Neutral",
                        Bias::Neutral,
                        Some(self.bias.clone()),
                        Message::Bias,
                    ))
                    .push(Radio::new(
                        "Bearish",
                        Bias::Bearish,
                        Some(self.bias.clone()),
                        Message::Bias,
                    ))
                    .spacing(20)
            )
            .spacing(10)
            .align_x(Alignment::Center);

        let status = text(message)
            .size(20);

        let content = iced::widget::column![
            control_buttons,
            risk_reward_slider,
            bias_controls,
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