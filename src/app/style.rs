
use ratatui::{
    text::Line,
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{canvas::Canvas, Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

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
