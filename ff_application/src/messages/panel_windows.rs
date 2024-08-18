use iced::widget::pane_grid;

#[derive(Clone, Debug)]
pub enum PaneWindowMessage {
    Split(iced::window::Id, pane_grid::Axis, pane_grid::Pane),
    SplitFocused(iced::window::Id, pane_grid::Axis),
    FocusAdjacent(iced::window::Id,pane_grid::Direction),
    Clicked(iced::window::Id, pane_grid::Pane),
    Dragged(iced::window::Id, pane_grid::DragEvent),
    Resized(iced::window::Id,pane_grid::ResizeEvent),
    TogglePin(iced::window::Id, pane_grid::Pane),
    Maximize(iced::window::Id,pane_grid::Pane),
    Restore(iced::window::Id),
    Close(iced::window::Id,pane_grid::Pane),
    CloseFocused(iced::window::Id),
}

impl PaneWindowMessage {
    pub fn window_id(&self) -> iced::window::Id {
        match &self {
            PaneWindowMessage::Split(id, _, _) => id.clone(),
            PaneWindowMessage::SplitFocused(id, _) =>  id.clone(),
            PaneWindowMessage::FocusAdjacent(id, _) => id.clone(),
            PaneWindowMessage::Clicked(id, _) =>  id.clone(),
            PaneWindowMessage::Dragged(id, _) => id.clone(),
            PaneWindowMessage::Resized(id, _) =>  id.clone(),
            PaneWindowMessage::TogglePin(id, _) =>  id.clone(),
            PaneWindowMessage::Maximize(id, _) => id.clone(),
            PaneWindowMessage::Restore(id) =>  id.clone(),
            PaneWindowMessage::Close(id, _) => id.clone(),
            PaneWindowMessage::CloseFocused(id) =>  id.clone(),
        }
    }
}