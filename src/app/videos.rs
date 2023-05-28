use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use invidious::structs::universal::Search;
use ratatui::widgets::{canvas::Line, Block, Borders, List, ListItem, ListState, Paragraph};
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
pub struct VideosList {
    pub state: ListState,
    pub search: Search,
}

impl Default for VideosList {
    fn default() -> VideosList {
        VideosList {
            state: ListState::default(),
            search: Search { items: Vec::new() },
        }
    }
}

impl VideosList {
    pub fn with_items(search: Search) -> VideosList {
        VideosList {
            state: ListState::default(),
            search,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.search.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.search.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
