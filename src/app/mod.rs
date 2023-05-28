use std::{error::Error, io};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{canvas::Line, Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Default)]
enum State {
    #[default]
    None,
    Search,
    List,
    Exit,
}

#[derive(Default)]
struct VideosList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> VideosList<T> {
    fn with_items(items: Vec<T>) -> VideosList<T> {
        VideosList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

#[derive(Default)]
pub struct App {
    state: State,
    input: String,
    items: VideosList<String>,
}

impl App {
    fn handle_event(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = State::Exit;
            }
            KeyCode::Char('/') => {
                self.state = State::Search;
            }
            _ => {}
        }
    }

    fn handle_event_search(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Esc => {
                self.state = State::None;
            }
            KeyCode::Enter => {
                self.state = State::None;
                // Search
            }
            _ => {}
        }
    }

    fn handle_event_list(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.state = State::None;
            }
            KeyCode::Enter => {
                self.state = State::None;
                // Open video
            }
            _ => {}
        }
    }

    pub fn exec<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match self.state {
                        State::None => self.handle_event(key.code),
                        State::Search => self.handle_event_search(key.code),
                        State::List => self.handle_event_list(key.code),
                        State::Exit => return Ok(()),
                    }
                }
            }
        }
    }

    fn ui<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Search"))
            .style(if self.state == State::Search {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });
        f.render_widget(input, chunks[0]);

        if self.state == State::Search {
            f.set_cursor(chunks[0].x + self.input.width() as u16 + 1, chunks[0].y + 1)
        }

        let items: Vec<ListItem> = self
            .items
            .items
            .iter()
            .map(|v| ListItem::new(v.as_str()).style(Style::default().fg(Color::Black).bg(Color::White)))
            .collect();

        let items = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Videos"))
            .style(if self.state == State::List {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");
        f.render_stateful_widget(items, chunks[1], &mut self.items.state);
    }
}
