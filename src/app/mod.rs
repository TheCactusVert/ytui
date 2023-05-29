mod videos;

use videos::VideosList;

use std::error::Error;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use invidious::reqwest::asynchronous::functions::search;
use invidious::reqwest::asynchronous::Client;
use invidious::structs::hidden::SearchItem::{Channel, Playlist, Unknown, Video};
use invidious::structs::universal::Search;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{canvas::Line, Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::either::Either;
use tokio_util::sync::CancellationToken;
use unicode_width::UnicodeWidthStr;

type SharedSearch = Arc<Mutex<VideosList>>;

#[derive(PartialEq, Default, Debug)]
enum State {
    #[default]
    List,
    Search,
    Exit,
}

pub struct App {
    state: State,
    input: String,
    list: VideosList,

    search: SharedSearch,
    rt: Runtime,
    thread: Option<(CancellationToken, JoinHandle<()>)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: State::default(),
            input: String::default(),
            list: VideosList::default(),
            search: Arc::new(Mutex::new(VideosList::default())),
            rt: Runtime::new().unwrap(),
            thread: None,
        }
    }
}

impl App {
    fn handle_event_list(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = State::Exit;
                self.stop();
            }
            KeyCode::Char('/') => {
                self.state = State::Search;
            }
            KeyCode::Enter => {
                // Open video
            }
            KeyCode::Up => {
                self.list.previous();
            }
            KeyCode::Down => {
                self.list.next();
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
                self.state = State::List;
            }
            KeyCode::Enter => {
                self.state = State::List;
                self.stop();
                self.start(self.input.clone());
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
                            State::List => self.handle_event_list(key.code),
                            State::Search => self.handle_event_search(key.code),
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

        let search = self.search.lock().unwrap();
        let items: Vec<ListItem> = search
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

        let videos_list = Self::ui_list(items);
        f.render_stateful_widget(videos_list, chunks[0], &mut self.list.state);

        if self.state == State::Search {
            let search_paragraph = Self::ui_search(self.input.as_str());
            let area = Self::centered_rect(60, 10, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(search_paragraph, area);
            f.set_cursor(area.x + self.input.width() as u16 + 1, area.y + 1);
        }
    }

    fn ui_list<'a, T>(items: T) -> List<'a>
    where
        T: Into<Vec<ListItem<'a>>>,
    {
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Videos"))
            .style(Style::default())
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ")
    }

    fn ui_search(input: &str) -> Paragraph {
        Paragraph::new(input)
            .block(Block::default().borders(Borders::ALL).title("Search"))
            .style(Style::default().fg(Color::Yellow))
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

    fn start(&mut self, input: String) {
        assert!(self.thread.is_none());

        let token = CancellationToken::new();
        let join = self
            .rt
            .spawn(Self::run(self.search.clone(), token.clone(), input));

        self.thread = Some((token, join));
    }

    fn stop(&mut self) {
        if let Some(mut thread) = self.thread.take() {
            thread.0.cancel();
            self.rt.block_on(&mut thread.1).unwrap();
        }

        *self.search.lock().unwrap() = VideosList::default();
    }

    async fn run(search: SharedSearch, token: CancellationToken, input: String) {
        let client = Client::new(String::from("https://vid.puffyan.us"));
        let input = format!("q={input}");
        let fetch = client.search(Some(&input));

        let result = select! {
            s = fetch => s,
            _ = token.cancelled() => return,
        };

        // Lock only when data is received
        if let Ok(s) = result {
            *search.lock().unwrap() = VideosList::with_items(s);
        }
    }
}
