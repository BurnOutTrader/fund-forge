use iced::Theme;

pub trait WindowTrait {
    // Getters
    fn title(&self) -> String;
    fn scale_input(&self) -> &str;
    fn current_scale(&self) -> f64;
    fn theme(&self) -> &Theme;
    fn input_id(&self) -> iced::widget::text_input::Id;
    // Setters
    fn set_title(&mut self, title: String);
    fn set_scale_input(&mut self, scale_input: String);
    fn set_current_scale(&mut self, current_scale: f64);
    // Note: Depending on how Theme is defined, you might want a different setter signature
    fn set_theme(&mut self, theme: Theme);
}
