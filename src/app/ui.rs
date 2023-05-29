use ratatui::style::{Color, Modifier, Style};

pub const STYLE_DEFAULT: Style = Style {
    fg: Some(Color::Reset),
    bg: Some(Color::Reset),
    add_modifier: Modifier::empty(),
    sub_modifier: Modifier::empty(),
};

pub const STYLE_TITLE: Style = Style {
    fg: Some(Color::Reset),
    bg: Some(Color::Reset),
    add_modifier: Modifier::BOLD,
    sub_modifier: Modifier::empty(),
};

pub const STYLE_HIGHLIGHT: Style = Style {
    fg: Some(Color::Red),
    bg: Some(Color::Reset),
    add_modifier: Modifier::empty(),
    sub_modifier: Modifier::empty(),
};

pub const STYLE_HIGHLIGHT_ITEM: Style = Style {
    fg: Some(Color::Reset),
    bg: Some(Color::Red),
    add_modifier: Modifier::empty(),
    sub_modifier: Modifier::empty(),
};
