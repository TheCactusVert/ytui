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
    widgets::{canvas::Line, Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Default)]
enum State {
    #[default]
    None,
    Search,
    Exit,
}

#[derive(Default)]
pub struct App {
    state: State,
    input: String,
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

    pub fn exec<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match self.state {
                        State::None => self.handle_event(key.code),
                        State::Search => self.handle_event_search(key.code),
                        State::Exit => return Ok(()),
                    }
                }
            }
        }
    }

    fn ui<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        let input = Paragraph::new(self.input.as_str())
            .style(if self.state == State::Search {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .block(Block::default().borders(Borders::ALL).title("Search"));
        f.render_widget(input, chunks[0]);

        if self.state == State::Search {
            f.set_cursor(chunks[0].x + self.input.width() as u16 + 1, chunks[0].y + 1)
        }
    }
}
