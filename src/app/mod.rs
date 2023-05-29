mod ui;

use crate::util;
use crate::Event;
use crate::EventSender;
use ui::*;

use std::process::Command;
use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use invidious::reqwest::asynchronous::Client;
use invidious::structs::universal::Search;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::{Block, Borders, List, ListState, Paragraph},
    Frame,
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
    rt: Runtime,
    event_tx: EventSender,
    search: String,
    result_search: SharedSearch,
    result_search_selection: ListState,
    searcher: Option<(CancellationToken, JoinHandle<()>)>,
}

impl App {
    pub fn new(event_tx: EventSender) -> Self {
        Self {
            state: State::default(),
            rt: Runtime::new().unwrap(),
            event_tx,
            search: String::default(),
            result_search: Arc::new(Mutex::new(Search { items: Vec::new() })),
            result_search_selection: ListState::default(),
            searcher: None,
        }
    }

    fn next_video(&mut self) {
        let result_search = self.result_search.lock().unwrap();

        if result_search.items.len() != 0 {
            let i = match self.result_search_selection.selected() {
                Some(i) if i == result_search.items.len() - 1 => result_search.items.len() - 1,
                Some(mut i) => {
                    i += 1;
                    i %= result_search.items.len();
                    i
                }
                None => 0,
            };
            self.result_search_selection.select(Some(i));
        } else {
            self.result_search_selection.select(None);
        }
    }

    fn previous_video(&mut self) {
        let result_search = self.result_search.lock().unwrap();

        if result_search.items.len() != 0 {
            let i = match self.result_search_selection.selected() {
                Some(i) if i == 0 => 0,
                Some(mut i) => {
                    i -= 1;
                    i %= result_search.items.len();
                    i
                }
                None => result_search.items.len() - 1,
            };
            self.result_search_selection.select(Some(i));
        } else {
            self.result_search_selection.select(None);
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
                self.search.push(c);
            }
            KeyCode::Backspace => {
                self.search.pop();
            }
            KeyCode::Esc => {
                self.state = State::List;
            }
            KeyCode::Enter => {
                self.state = State::List;
                self.stop_search();
                self.start_search(self.search.clone());
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

        let search_paragraph = Paragraph::new(self.search.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(search_title)
                .border_style(self.get_border_style(State::Search)),
        );
        f.render_widget(search_paragraph, chunks_a[0]);
        if self.state == State::Search {
            f.set_cursor(
                chunks_a[0].x + self.search.width() as u16 + 1,
                chunks_a[0].y + 1,
            );
        }

        let chunks_b = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(chunks_a[1]);

        let result_search = self.result_search.lock().unwrap();
        let result_items = util::search_to_list_items(&result_search);
        let result_list = List::new(result_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(result_title)
                    .border_style(self.get_border_style(State::List)),
            )
            .highlight_style(STYLE_HIGHLIGHT_ITEM);
        f.render_stateful_widget(result_list, chunks_b[0], &mut self.result_search_selection);
    }

    fn start_search(&mut self, search: String) {
        assert!(self.searcher.is_none());

        let token = CancellationToken::new();
        let join = self.rt.spawn(Self::run_search(
            self.event_tx.clone(),
            self.result_search.clone(),
            token.clone(),
            search,
        ));

        self.searcher = Some((token, join));
    }

    fn stop_search(&mut self) {
        if let Some(mut thread) = self.searcher.take() {
            thread.0.cancel();
            self.rt.block_on(&mut thread.1).unwrap();
        }

        let mut result_search = self.result_search.lock().unwrap();
        result_search.items.clear();
        self.result_search_selection.select(None);
    }

    async fn run_search(
        event_tx: EventSender,
        result_search: SharedSearch,
        token: CancellationToken,
        search: String,
    ) {
        let client = Client::new(String::from("https://vid.puffyan.us"));
        let search = format!("q={search}");
        let fetch = client.search(Some(&search));

        let result = select! {
            s = fetch => s,
            _ = token.cancelled() => return,
        };

        if let Ok(s) = result {
            *result_search.lock().unwrap() = s;
        }

        event_tx.send(Event::Fetch).unwrap();
    }
}
