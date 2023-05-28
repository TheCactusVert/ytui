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

#[derive(Default)]
pub struct App {
    searching: bool,
    input: String,
}

impl App {
    pub fn exec<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if self.searching {
                        match key.code {
                            KeyCode::Char(c) if self.searching => {
                                self.input.push(c);
                            }
                            KeyCode::Backspace if self.searching => {
                                self.input.pop();
                            }
                            KeyCode::Esc => {
                                self.searching = false;
                            }
                            KeyCode::Enter => {
                                self.searching = false;
                                // Search
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                return Ok(());
                            }
                            KeyCode::Char('/') => {
                                self.searching = true;
                            }
                            _ => {}
                        }
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
            .style(if self.searching {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .block(Block::default().borders(Borders::ALL).title("Search"));
        f.render_widget(input, chunks[0]);

        if self.searching {
            f.set_cursor(chunks[0].x + self.input.width() as u16 + 1, chunks[0].y + 1)
        }
    }
}
