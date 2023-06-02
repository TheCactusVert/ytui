mod ui;
mod widgets;
pub mod search;

use crate::util;
use crate::Event;
use crate::EventSender;
use ui::*;
use widgets::Image;
use search::Search;

use std::convert::AsRef;
use std::io::Cursor;
use std::process::Command;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use image::io::Reader as ImageReader;
use invidious::reqwest::asynchronous::Client;
use invidious::structs::hidden::SearchItem::*;
use invidious::structs::universal::Search as InvidiousSearch;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{
        canvas::{Canvas, Points},
        Block, Borders, List, ListState, Paragraph, Wrap,
    },
    Frame,
};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use unicode_width::UnicodeWidthStr;
use which::which;

type SharedSearch = Arc<Mutex<Vec<Search>>>;

#[derive(PartialEq, Default, Debug)]
enum State {
    #[default]
    List,
    Search,
    Item,
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
            result_search: Arc::new(Mutex::new(Vec::new())),
            result_search_selection: ListState::default(),
            searcher: None,
        }
    }

    fn next_video(&mut self) {
        let result_search = self.result_search.lock().unwrap();

        if result_search.len() != 0 {
            let i = match self.result_search_selection.selected() {
                Some(i) if i == result_search.len() - 1 => result_search.len() - 1,
                Some(mut i) => {
                    i += 1;
                    i %= result_search.len();
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

        if result_search.len() != 0 {
            let i = match self.result_search_selection.selected() {
                Some(i) if i == 0 => 0,
                Some(mut i) => {
                    i -= 1;
                    i %= result_search.len();
                    i
                }
                None => result_search.len() - 1,
            };
            self.result_search_selection.select(Some(i));
        } else {
            self.result_search_selection.select(None);
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
            KeyCode::Char('k') | KeyCode::Up => self.previous_video(),
            KeyCode::Char('j') | KeyCode::Down => self.next_video(),
            KeyCode::Tab => {
                self.state = State::Item;
            }
            _ => {}
        }
    }

    fn handle_event_item(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = State::Exit;
                self.stop_search();
            }
            KeyCode::Char('/') => {
                self.state = State::Search;
            }
            KeyCode::Tab => {
                self.state = State::List;
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
                State::Item => self.handle_event_item(key.code),
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
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
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

        if let Some(i) = self.result_search_selection.selected() {
            match &result_search[i] {
                Search::Video { title, author, .. } => self.ui_video(f, chunks_b[1], &title, &author),
                Search::Playlist { title, author, .. } => self.ui_playlist(f, chunks_b[1], &title, &author),
                Search::Channel {
                    name, description, ..
                } => self.ui_channel(f, chunks_b[1], &name, &description),
                Search::Unknown { .. } => self.ui_empty(f, chunks_b[1]),
            }
        } else {
            self.ui_empty(f, chunks_b[1]);
        }
    }

    fn ui_video<B: Backend>(&self, f: &mut Frame<B>, rect: Rect, title: &str, author: &str) {
        let mut video_title = Line::from("Video");
        video_title.patch_style(STYLE_TITLE);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(video_title)
            .border_style(self.get_border_style(State::Item));
        f.render_widget(block, rect);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Ratio(9, 16),
                    Constraint::Min(1),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(rect);

        // TODO should be thumbnail
        let thumbnail = ImageReader::new(Cursor::new(include_bytes!("../../static/thumbnail.jpg")))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        let thumbnail = Image::new(&thumbnail);
        f.render_widget(thumbnail, chunks[0]);

        let title = Paragraph::new(title).style(STYLE_TITLE);
        f.render_widget(title, chunks[1]);

        let author = Paragraph::new(author).style(STYLE_AUTHOR);
        f.render_widget(author, chunks[2]);
    }

    fn ui_playlist<B: Backend>(&self, f: &mut Frame<B>, rect: Rect, title: &str, author: &str) {
        let mut playlist_title = Line::from("Playlist");
        playlist_title.patch_style(STYLE_TITLE);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(playlist_title)
            .border_style(self.get_border_style(State::Item));
        f.render_widget(block, rect);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(1), Constraint::Min(1)].as_ref())
            .split(rect);

        let title = Paragraph::new(title).style(STYLE_TITLE);
        f.render_widget(title, chunks[0]);

        let author = Paragraph::new(author).style(STYLE_AUTHOR);
        f.render_widget(author, chunks[1]);
    }

    fn ui_channel<B: Backend>(&self, f: &mut Frame<B>, rect: Rect, name: &str, description: &str) {
        let mut channel_title = Line::from("Channel");
        channel_title.patch_style(STYLE_TITLE);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(channel_title)
            .border_style(self.get_border_style(State::Item));
        f.render_widget(block, rect);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(16),
                    Constraint::Min(1),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(rect);

        // TODO should be thumbnail
        let thumbnail = ImageReader::new(Cursor::new(include_bytes!("../../static/channel.jpg")))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        let thumbnail = Image::new(&thumbnail);
        f.render_widget(thumbnail, chunks[0]);

        let name = Paragraph::new(name).style(STYLE_TITLE);
        f.render_widget(name, chunks[1]);

        let description = Paragraph::new(description)
            .style(STYLE_AUTHOR)
            .wrap(Wrap { trim: true });
        f.render_widget(description, chunks[2]);
    }

    fn ui_empty<B: Backend>(&self, f: &mut Frame<B>, rect: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.get_border_style(State::Item));
        f.render_widget(block, rect);
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
        result_search.clear();
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

        let result = match result {
            Ok(r) => r,
            Err(_) => return,
        };

        let result: Vec<Search> = result.items.into_iter().map(|i| i.into()).collect();

        *result_search.lock().unwrap() = result;

        event_tx.send(Event::Fetch).unwrap();
    }
}
