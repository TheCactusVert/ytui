mod videos;
mod worker;

use videos::VideosList;
use worker::Worker;

use std::time::Duration;
use std::{error::Error, io};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use invidious::structs::hidden::SearchItem::{Channel, Playlist, Unknown, Video};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{canvas::Line, Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Default, Debug)]
enum State {
    #[default]
    None,
    Search,
    List,
    Exit,
}

#[derive(Default)]
pub struct App {
    state: State,
    input: String,
    worker: Worker,
    list: VideosList,
}

impl App {
    fn handle_event(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = State::Exit;
                self.worker.stop();
            }
            KeyCode::Char('/') => {
                self.state = State::Search;
                self.worker.stop();
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
                self.worker.start(self.input.clone());
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
        while self.state != State::Exit {
            terminal.draw(|f| self.ui(f))?;

            if crossterm::event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match self.state {
                            State::None => self.handle_event(key.code),
                            State::Search => self.handle_event_search(key.code),
                            State::List => self.handle_event_list(key.code),
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn ui<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        let mut search = self.worker.get_search();

        self.list = VideosList::with_items(search);

        let items: Vec<ListItem> = self
            .list
            .search
            .items
            .iter()
            .map(|v| {
                ListItem::new(match v {
                    Video { title, .. } => title.as_str(),
                    Playlist { title, .. } => title.as_str(),
                    Channel { name, .. } => name.as_str(),
                    Unknown(_) => "Error",
                })
                .style(Style::default())
            })
            .collect();

        let videos_list = List::new(items)
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
        f.render_stateful_widget(videos_list, chunks[0], &mut self.list.state);

        if self.state == State::Search {
            self.ui_search(f);
        }
    }

    fn ui_search<B: Backend>(&mut self, f: &mut Frame<B>) {
        let search_paragraph = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Search"))
            .style(Style::default().fg(Color::Yellow));
        let area = Self::centered_rect(60, 20, f.size());
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(search_paragraph, area);
        f.set_cursor(area.x + self.input.width() as u16 + 1, area.y + 1);
    }

    /// helper function to create a centered rect using up certain percentage of the available rect `r`
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                .as_ref(),
            )
            .split(popup_layout[1])[1]
    }
}
