use std::collections::{BTreeMap};
use std::str::FromStr;
use std::sync::Arc;
use chrono::{DateTime, FixedOffset};
use chrono_tz::Tz;
use iced::alignment::{self, Alignment};
use iced::{window};
use iced::theme::{self, Theme};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{
    button, column, container, responsive, row, scrollable, text,
};
use iced::{
    Color, Element, Length, Size,
};
use tokio::sync::{Mutex};
use ff_standard_lib::standardized_types::data_server_messaging::AddressString;
use ff_standard_lib::standardized_types::OwnerId;
use ff_standard_lib::standardized_types::subscriptions::DataSubscription;
use crate::application::canvases::charting::canvas::graph::canvas::SeriesCanvas;
use crate::application::canvases::charting::canvas::graph::models::data::SeriesData;
use crate::application::traits::WindowTrait;
use crate::messages::application::ApplicationMessages;
use crate::messages::panel_windows::PaneWindowMessage;


#[derive(Debug, Clone)]
pub struct StrategyWindow {
    //for canvas management
    //pub canvases: BTreeMap<Subscription, SeriesCanvas>,
    pub address: AddressString,
    pub owner_id: OwnerId,

    //for pane grid management
    pub panes: pane_grid::State<Pane>,
    pub panes_created:  usize,
    pub focus:  Option<pane_grid::Pane>,

    //for window management
    title: String,
    scale_input: String,
    current_scale: f64,
    theme: Theme,
    input_id: iced::widget::text_input::Id,
    time_zone: Tz,
    last_time: DateTime<FixedOffset>,
    start_time: DateTime<FixedOffset>,
    
    //data
    charts: Arc<Mutex<BTreeMap<DataSubscription, Vec<SeriesCanvas>>>>,
    subscriptions: Vec<DataSubscription>,
}

impl StrategyWindow {
    pub async fn add_data(&mut self, subscription: DataSubscription, time: i64, data: Vec<SeriesData>) {
        for canvas in self.charts.lock().await.get_mut(&subscription).unwrap() {
            canvas.add_data( time, data.clone());
        }
    }
    
    pub fn new(owner_id: OwnerId, address: AddressString, theme: Theme, time_zone_string: String, start_time: String, last_time: String) -> Self {
        let (panes, _) = pane_grid::State::new(Pane::new(1));
        let time_zone = Tz::from_str(&time_zone_string).unwrap();
       
        StrategyWindow {
            title: format!("{} at {}", owner_id.to_string(), address.to_string()),
            address,
            time_zone,
            last_time: DateTime::from_str(&last_time).unwrap(),
            start_time: DateTime::from_str(&start_time).unwrap(),
            owner_id,
            panes,
            panes_created: 0,
            focus: None,
            scale_input: "1.0".to_string(),
            current_scale: 1.0,
            theme,
            input_id: iced::widget::text_input::Id::new("1"),
            charts: Arc::new(Mutex::new(BTreeMap::new())),
            subscriptions: Vec::new(),
        }
    }

    pub fn update(&mut self, message: PaneWindowMessage)  {
        match message {
            PaneWindowMessage::Split(_, axis, pane) => {
                let result =
                    self.panes.split(axis, pane, Pane::new(self.panes_created));

                if let Some((pane, _)) = result {
                    self.focus = Some(pane);
                }

                self.panes_created += 1;
            }
            PaneWindowMessage::SplitFocused(_, axis) => {
                if let Some(pane) = self.focus {
                    let result = self.panes.split(
                        axis,
                        pane,
                        Pane::new(self.panes_created),
                    );

                    if let Some((pane, _)) = result {
                        self.focus = Some(pane);
                    }

                    self.panes_created += 1;
                }
            }
            PaneWindowMessage::FocusAdjacent(_, direction) => {
                if let Some(pane) = self.focus {
                    if let Some(adjacent) = self.panes.adjacent(pane, direction)
                    {
                        self.focus = Some(adjacent);
                    }
                }
            }
            PaneWindowMessage::Clicked(_, pane) => {
                self.focus = Some(pane);
            }
            PaneWindowMessage::Resized(_, pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
            }
            PaneWindowMessage::Dragged(_, pane_grid::DragEvent::Dropped {
                                 pane,
                                 target,
                             }) => {
                self.panes.drop(pane, target);
            }
            PaneWindowMessage::Dragged(_, _) => {}
            PaneWindowMessage::TogglePin(window, pane) => {
                if let Some(Pane { is_pinned, .. }) = self.panes.get_mut(pane) {
                    *is_pinned = !*is_pinned;
                }
            }
            PaneWindowMessage::Maximize(_, pane) => self.panes.maximize(pane),
            PaneWindowMessage::Restore(window, ) => {
                self.panes.restore();
            }
            PaneWindowMessage::Close(_, pane) => {
                if let Some((_, sibling)) = self.panes.close(pane) {
                    self.focus = Some(sibling);
                }
            }
            PaneWindowMessage::CloseFocused(window, ) => {
                if let Some(pane) = self.focus {
                    if let Some(Pane { is_pinned, .. }) = self.panes.get(pane) {
                        if !is_pinned {
                            if let Some((_, sibling)) = self.panes.close(pane) {
                                self.focus = Some(sibling);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn view(&self, window_id: window::Id) -> Element<ApplicationMessages> {
        let focus = self.focus;
        let total_panes = self.panes.len();

        let pane_grid = PaneGrid::new(&self.panes, |id, pane, is_maximized| {
            let is_focused = focus == Some(id);

            let pin_button = button(
                text(if pane.is_pinned { "Unpin" } else { "Pin" }).size(14),
            )
                .on_press(ApplicationMessages::PaneWindowMessages(PaneWindowMessage::TogglePin(window_id, id)))
                .padding(3);

            let title = row![
                pin_button,
                "Pane",
                text(pane.id.to_string()).style(if is_focused {
                    PANE_ID_COLOR_FOCUSED
                } else {
                    PANE_ID_COLOR_UNFOCUSED
                }),
            ]
                .spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(view_controls(id, total_panes, pane.is_pinned, is_maximized, window_id))
                .padding(10)
                .style(if is_focused {
                    style::title_bar_focused
                } else {
                    style::title_bar_active
                });

            pane_grid::Content::new(responsive(move |size| {
                view_content(id, total_panes, pane.is_pinned, size,window_id)
            }))
                .title_bar(title_bar)
                .style(if is_focused {
                    style::pane_focused
                } else {
                    style::pane_active
                })
        })
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(10)
            .on_click(move |pane| ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Clicked(window_id, pane)))
            .on_drag(move |event| ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Dragged(window_id, event)))
            .on_resize(10, move |event| ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Resized(window_id, event)));

        container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }
}

impl WindowTrait for StrategyWindow {
    fn title(&self) -> String {
      self.title.clone()
    }

    fn scale_input(&self) -> &str {
        &self.scale_input
    }

    fn current_scale(&self) -> f64 {
        self.current_scale.clone()
    }

    fn theme(&self) -> &Theme {
        &self.theme
    }

    fn input_id(&self) -> iced::widget::text_input::Id {
        self.input_id.clone()
    }

    fn set_title(&mut self, title: String) {
        self.title = title;
    }

    fn set_scale_input(&mut self, scale_input: String) {
        self.scale_input = scale_input;
    }

    fn set_current_scale(&mut self, current_scale: f64) {
        self.current_scale = current_scale;
    }

    fn set_theme(&mut self, theme: Theme) {
       self.theme = theme;
    }
}

const PANE_ID_COLOR_UNFOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0xC7 as f32 / 255.0,
    0xC7 as f32 / 255.0,
);
const PANE_ID_COLOR_FOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0x47 as f32 / 255.0,
    0x47 as f32 / 255.0,
);


fn view_content<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_pinned: bool,
    size: Size,
    window_id: iced::window::Id
) -> Element<'a, ApplicationMessages> {
    let button = |label, message| {
        button(
            text(label)
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(16),
        )
            .width(Length::Fill)
            .padding(8)
            .on_press(message)
    };

    let controls = column![
        button(
            "Split horizontally",
            ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Split(window_id.clone(), pane_grid::Axis::Horizontal, pane)),
        ),
        button(
            "Split vertically",
            ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Split(window_id, pane_grid::Axis::Vertical, pane)),
        )
    ]
        .push_maybe(if total_panes > 1 && !is_pinned {
            Some(
                button("Close", ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Close(window_id, pane)))
                    .style(theme::Button::Destructive),
            )
        } else {
            None
        })
        .spacing(5)
        .max_width(160);

    let content = column![
        text(format!("{}x{}", size.width, size.height)).size(24),
        controls,
    ]
        .spacing(10)
        .align_items(Alignment::Center);

    container(scrollable(content))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(1)
        .center_y()
        .into()
}

fn view_controls<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_pinned: bool,
    is_maximized: bool,
    window_id: iced::window::Id
) -> Element<'a, ApplicationMessages> {
    let row = row![].spacing(5).push_maybe(if total_panes > 1 {
        let (content, message) = if is_maximized {
            ("Restore", ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Restore(window_id)))
        } else {
            ("Maximize", ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Maximize(window_id, pane)))
        };

        Some(
            button(text(content).size(14))
                .style(theme::Button::Secondary)
                .padding(3)
                .on_press(message),
        )
    } else {
        None
    });

    let close = button(text("Close").size(14))
        .style(theme::Button::Destructive)
        .padding(3)
        .on_press_maybe(if total_panes > 1 && !is_pinned {
            Some(ApplicationMessages::PaneWindowMessages(PaneWindowMessage::Close(window_id, pane)))
        } else {
            None
        });

    row.push(close).into()
}

mod style {
    use iced::widget::container;
    use iced::{Border, Theme};

    pub fn title_bar_active(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.primary.strong.text),
            background: Some(palette.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane_active(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Border::default()
            },
            ..Default::default()
        }
    }

    pub fn pane_focused(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.primary.strong.color,
                ..Border::default()
            },
            ..Default::default()
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub(crate) struct Pane {
    id: usize,
    pub is_pinned: bool,
}

impl Pane {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            is_pinned: false,
        }
    }
}

/*async fn send_message(mut stream: Arc<Mutex<TcpStream>>, message: StrategyMessages) {
    let bytes = message.to_bytes();
    // Length of the message as a 4-byte array
    let length = (bytes.len() as u32).to_be_bytes(); // Use big-endian format
    let mut stream = stream.lock().await;
    // Send the length header
    match stream.write_all(&length).await {
        Ok(_) => {},
        Err(e) => {
            println!("Error sending message: {}", e);
            return;
        }
    }

    // Send the actual message
    match stream.write_all(&bytes).await {
        Ok(_) => {},
        Err(e) => {
            println!("Error sending message: {}", e);
            return;
        }
    }
}*/
