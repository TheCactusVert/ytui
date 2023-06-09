use crate::app::search::Search;

use crossterm::event::{KeyEvent, MouseEvent};

pub enum Event {
    FocusGained,
    FocusLost,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    Resize(u16, u16),
    Fetch(Search),
}

impl From<crossterm::event::Event> for Event {
    fn from(event: crossterm::event::Event) -> Self {
        match event {
            crossterm::event::Event::FocusGained => Event::FocusGained,
            crossterm::event::Event::FocusLost => Event::FocusLost,
            crossterm::event::Event::Key(k) => Event::Key(k),
            crossterm::event::Event::Mouse(m) => Event::Mouse(m),
            crossterm::event::Event::Paste(p) => Event::Paste(p),
            crossterm::event::Event::Resize(w, h) => Event::Resize(w, h),
        }
    }
}
