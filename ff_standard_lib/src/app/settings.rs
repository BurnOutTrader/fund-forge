use rkyv::{Archive, Deserialize as Deserialize_rkyv, Serialize as Serialize_rkyv};

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub enum ColorTheme {
    Dark,
    Light,
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct ColorTemplate {
    /// Red component, 0.0 - 1.0
    pub r: f32,
    /// Green component, 0.0 - 1.0
    pub g: f32,
    /// Blue component, 0.0 - 1.0
    pub b: f32,
    /// Transparency, 0.0 - 1.0
    pub a: f32,
}

impl ColorTemplate {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> ColorTemplate {
        ColorTemplate { r, g, b, a }
    }
}

/// `DisplaySettings` is a struct that holds the configuration for the display of a chart element.
///
/// # Properties
/// * `color`: The color of the chart element.
/// * `size`: The size of the chart element.
/// * `show`: A boolean indicating whether the chart element should be shown.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct DisplaySettings {
    /// The color of the chart element.
    pub color: ColorTemplate,
    /// The size of the chart element.
    pub size: f32,
    /// A boolean indicating whether the chart element should be shown.
    pub show: bool,
}

impl DisplaySettings {
    pub fn from_color_theme(color_theme: ColorTheme) -> DisplaySettings {
        match color_theme {
            ColorTheme::Dark => DisplaySettings::dark_mode_settings(),
            ColorTheme::Light => DisplaySettings::light_mode_settings(),
        }
    }

    pub fn light_mode_settings() -> DisplaySettings {
        DisplaySettings::new(ColorTemplate::new(0.01, 0.01, 0.01, 0.2), 1.0, true)
    }

    pub fn dark_mode_settings() -> DisplaySettings {
        DisplaySettings::new(ColorTemplate::new(0.24, 0.24, 0.24, 0.4), 1.0, true)
    }

    pub fn new(color: ColorTemplate, size: f32, show: bool) -> Self {
        DisplaySettings { color, size, show }
    }
}

impl Default for DisplaySettings {
    fn default() -> Self {
        DisplaySettings {
            show: true,
            color: ColorTemplate::new(0.24, 0.24, 0.24, 0.4),
            size: 1.0,
        }
    }
}

/// `TextSettings` is a struct that holds the configuration for the display of text in a chart.
///
/// # Properties
/// * `color`: The color of the text.
/// * `size`: The size of the text.
/// * `show`: A boolean indicating whether the text should be shown.
/// * `style`: The style of the font used for the text.
#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct TextSettings {
    /// The color of the text.
    pub color: ColorTemplate,
    /// The size of the text.
    pub size: f32,
    /// A boolean indicating whether the text should be shown.
    pub show: bool,
}

impl TextSettings {
    pub fn from_color_theme(color_theme: ColorTheme) -> TextSettings {
        match color_theme {
            ColorTheme::Dark => TextSettings::dark_mode_settings(),
            ColorTheme::Light => TextSettings::light_mode_settings(),
        }
    }

    pub fn new(color: ColorTemplate, size: f32, show: bool) -> Self {
        TextSettings { color, size, show }
    }

    pub fn light_mode_settings() -> TextSettings {
        TextSettings::new(ColorTemplate::new(0.1, 0.1, 0.1, 1.0), 12.0, true)
    }

    pub fn dark_mode_settings() -> TextSettings {
        TextSettings::new(ColorTemplate::new(0.7, 0.7, 0.7, 1.0), 12.0, true)
    }
}

impl Default for TextSettings {
    fn default() -> Self {
        TextSettings {
            color: ColorTemplate::new(0.24, 0.24, 0.24, 1.0),
            size: 12.0,
            show: true,
        }
    }
}

#[derive(Clone, Serialize_rkyv, Deserialize_rkyv, Archive, PartialEq, Debug)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug))]
pub struct GraphElementSettings {
    /// The settings for the chart element.
    pub object_settings: DisplaySettings,
    /// The settings for the text of the chart element.
    pub text_settings: TextSettings,
}

impl Default for GraphElementSettings {
    fn default() -> Self {
        GraphElementSettings::light_mode_settings()
    }
}

impl GraphElementSettings {
    pub fn light_mode_settings() -> GraphElementSettings {
        let object_settings = DisplaySettings::light_mode_settings();
        let text_settings = TextSettings::light_mode_settings();
        GraphElementSettings {
            object_settings,
            text_settings,
        }
    }

    pub fn dark_mode_settings() -> GraphElementSettings {
        let object_settings = DisplaySettings::dark_mode_settings();
        let text_settings = TextSettings::dark_mode_settings();
        GraphElementSettings {
            object_settings,
            text_settings,
        }
    }

    pub fn new(object_settings: DisplaySettings, text_settings: TextSettings) -> Self {
        GraphElementSettings {
            object_settings,
            text_settings,
        }
    }
}
