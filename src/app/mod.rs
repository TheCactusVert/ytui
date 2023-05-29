mod style;

use crate::util;
use crate::Event;
use crate::EventSender;
use style::*;

use std::io;
use std::process::Command;
use std::sync::{Arc, Mutex};

use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind};
use invidious::reqwest::asynchronous::Client;
use invidious::structs::hidden::SearchItem::{Channel, Playlist, Unknown, Video};
use invidious::structs::universal::Search;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{canvas::Canvas, Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use unicode_width::UnicodeWidthStr;
use which::which;

type SharedSearch = Arc<Mutex<Search>>;

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
    rt: Runtime,
    event_tx: EventSender,
    search: SharedSearch,
    search_selection: ListState,
    searcher: Option<(CancellationToken, JoinHandle<()>)>,
}

impl App {
    pub fn new(event_tx: EventSender) -> Self {
        Self {
            state: State::default(),
            input: String::default(),
            rt: Runtime::new().unwrap(),
            event_tx,
            search: Arc::new(Mutex::new(Search { items: Vec::new() })),
            search_selection: ListState::default(),
            searcher: None,
        }
    }

    fn next_video(&mut self) {
        let search = self.search.lock().unwrap();

        if search.items.len() != 0 {
            let i = match self.search_selection.selected() {
                Some(i) if i == search.items.len() - 1 => search.items.len() - 1,
                Some(mut i) => {
                    i += 1;
                    i %= search.items.len();
                    i
                }
                None => 0,
            };
            self.search_selection.select(Some(i));
        } else {
            self.search_selection.select(None);
        }
    }

    fn previous_video(&mut self) {
        let search = self.search.lock().unwrap();

        if search.items.len() != 0 {
            let i = match self.search_selection.selected() {
                Some(i) if i == 0 => 0,
                Some(mut i) => {
                    i -= 1;
                    i %= search.items.len();
                    i
                }
                None => search.items.len() - 1,
            };
            self.search_selection.select(Some(i));
        } else {
            self.search_selection.select(None);
        }
    }

    fn handle_event_list(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = State::Exit;
                self.stop_search();
            }
            KeyCode::Char('/') => {
                self.state = State::Search;
            }
            KeyCode::Enter => match which("celluloid").or_else(|_| which("mpv")) {
                Ok(p) => {}
                Err(e) => {}
            },
            KeyCode::Up => self.previous_video(),
            KeyCode::Down => self.next_video(),
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
                self.stop_search();
                self.start_search(self.input.clone());
            }
            _ => {}
        }
    }

    pub fn is_running(&self) -> bool {
        self.state != State::Exit
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Press {
            match self.state {
                State::List => self.handle_event_list(key.code),
                State::Search => self.handle_event_search(key.code),
                _ => {}
            }
        }
    }

    fn get_border_style(&self, state: State) -> Style {
        if self.state == state {
            STYLE_HIGHLIGHT
        } else {
            STYLE_DEFAULT
        }
    }

    pub fn ui<B: Backend>(&mut self, f: &mut Frame<B>) {
        let mut search_title = Line::from("Search");
        search_title.patch_style(STYLE_TITLE);
        let mut result_title = Line::from("Results");
        result_title.patch_style(STYLE_TITLE);

        let chunks_a = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)].as_ref())
            .split(f.size());

        let search_paragraph = Paragraph::new(self.input.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(search_title)
                .border_style(self.get_border_style(State::Search)),
        );
        f.render_widget(search_paragraph, chunks_a[0]);
        if self.state == State::Search {
            f.set_cursor(
                chunks_a[0].x + self.input.width() as u16 + 1,
                chunks_a[0].y + 1,
            );
        }

        let chunks_b = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(chunks_a[1]);

        let search = self.search.lock().unwrap();
        let videos_list = List::new(util::search_to_list_items(&search))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(result_title)
                    .border_style(self.get_border_style(State::List)),
            )
            .highlight_style(STYLE_HIGHLIGHT_ITEM);
        f.render_stateful_widget(videos_list, chunks_b[0], &mut self.search_selection);
    }

    fn start_search(&mut self, input: String) {
        assert!(self.searcher.is_none());

        let token = CancellationToken::new();
        let join = self.rt.spawn(Self::run_search(
            self.event_tx.clone(),
            self.search.clone(),
            token.clone(),
            input,
        ));

        self.searcher = Some((token, join));
    }

    fn stop_search(&mut self) {
        if let Some(mut thread) = self.searcher.take() {
            thread.0.cancel();
            self.rt.block_on(&mut thread.1).unwrap();
        }

        let mut search = self.search.lock().unwrap();
        search.items.clear();
        self.search_selection.select(None);
    }

    async fn run_search(
        event_tx: EventSender,
        search: SharedSearch,
        token: CancellationToken,
        input: String,
    ) {
        let client = Client::new(String::from("https://vid.puffyan.us"));
        let input = format!("q={input}");
        let fetch = client.search(Some(&input));

        let result = select! {
            s = fetch => s,
            _ = token.cancelled() => return,
        };

        if let Ok(s) = result {
            *search.lock().unwrap() = s;
        }

        event_tx.send(Event::Fetch).unwrap();
    }
}
