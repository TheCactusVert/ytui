mod player;
pub mod search;
mod ui;
mod widgets;

use crate::Event;
use crate::EventSender;
use player::Player;
use search::Search;
use ui::*;
use widgets::Image;

use std::convert::AsRef;
use std::error::Error;
use std::io::Cursor;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use image::io::Reader as ImageReader;
use image::DynamicImage;
use invidious::hidden::SearchItem::{self, *};
use invidious::ClientAsync as Client;
use invidious::MethodAsync;
use ratatui::{
    backend::Backend,
    layout::Alignment,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, List, Paragraph, Wrap},
    Frame,
};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Default, Debug)]
enum State {
    #[default]
    List,
    Search,
    Item,
}

pub struct App {
    running: bool,
    state: State,
    rt: Runtime,
    event_tx: EventSender,
    input: String,
    search: Search,
    searcher: Option<(CancellationToken, JoinHandle<()>)>,
    player: Player,
}

impl App {
    pub fn new(event_tx: EventSender) -> Self {
        Self {
            running: true,
            state: State::default(),
            rt: Runtime::new().unwrap(),
            event_tx,
            input: String::default(),
            search: Search::default(),
            searcher: None,
            player: Player::new(),
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

    fn handle_event_list(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
                self.stop_search();
            }
            KeyCode::Char('/') => {
                self.state = State::Search;
            }
            KeyCode::Enter => match self.search.selected_item() {
                Some((Video { id, .. }, _)) => {
                    self.player.play_video(id);
                }
                _ => {}
            },
            KeyCode::Char('k') | KeyCode::Up => {
                self.search.previous_video();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.search.next_video();
            }
            KeyCode::Tab => {
                self.state = State::Item;
            }
            _ => {}
        }
    }

    fn handle_event_item(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
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
        self.running
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Press {
            match self.state {
                State::List => self.handle_event_list(key.code),
                State::Search => self.handle_event_search(key.code),
                State::Item => self.handle_event_item(key.code),
            }
        }
    }

    pub fn handle_fetch_event(&mut self, search: Search) {
        self.search = search;
    }

    pub fn handle_thumbnail_event(&mut self, i: usize, thumbnail: DynamicImage) {
        self.search.set_thumbnail(i, thumbnail);
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
            f.set_cursor(chunks_a[0].x + self.input.width() as u16 + 1, chunks_a[0].y + 1);
        }

        let chunks_b = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)].as_ref())
            .split(chunks_a[1]);

        let border = self.get_border_style(State::List);
        let mut list_split = self.search.get_list_split();
        let result_list = List::new(list_split.0)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(result_title)
                    .border_style(border),
            )
            .highlight_style(STYLE_HIGHLIGHT_ITEM);
        f.render_stateful_widget(result_list, chunks_b[0], &mut list_split.1);

        match self.search.selected_item() {
            Some((Video { title, author, .. }, thumbnail)) => {
                self.ui_video(f, chunks_b[1], title, author, thumbnail);
            }
            Some((Playlist { title, author, .. }, thumbnail)) => {
                self.ui_playlist(f, chunks_b[1], title, author);
            }
            Some((Channel { name, description, .. }, thumbnail)) => {
                self.ui_channel(f, chunks_b[1], name, description, thumbnail);
            }
            _ => {
                self.ui_empty(f, chunks_b[1]);
            }
        }
    }

    fn ui_video<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
        title: &str,
        author: &str,
        thumbnail: &Option<DynamicImage>,
    ) {
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
            .constraints([Constraint::Ratio(9, 16), Constraint::Min(1), Constraint::Min(1)].as_ref())
            .split(rect);

        match thumbnail {
            Some(thumbnail) => f.render_widget(Image::new(&thumbnail), chunks[0]),
            None => f.render_widget(
                Paragraph::new("Thumbnail loading...").alignment(Alignment::Center),
                chunks[0],
            ),
        };

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

    fn ui_channel<B: Backend>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
        name: &str,
        description: &str,
        thumbnail: &Option<DynamicImage>,
    ) {
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
            .constraints([Constraint::Length(16), Constraint::Min(1), Constraint::Min(1)].as_ref())
            .split(rect);

        match thumbnail {
            Some(thumbnail) => f.render_widget(Image::new(&thumbnail), chunks[0]),
            None => f.render_widget(
                Paragraph::new("Thumbnail loading...").alignment(Alignment::Center),
                chunks[0],
            ),
        };

        let name = Paragraph::new(name).style(STYLE_TITLE);
        f.render_widget(name, chunks[1]);

        let description = Paragraph::new(description)
            .style(STYLE_AUTHOR)
            .wrap(Wrap { trim: true });
        f.render_widget(description, chunks[2]);
    }

    fn ui_empty<B: Backend>(&self, f: &mut Frame<B>, rect: Rect) {
        let help = Paragraph::new("Hello World!").alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.get_border_style(State::Item)),
        );
        f.render_widget(help, rect);
    }

    fn start_search(&mut self, input: String) {
        assert!(self.searcher.is_none());

        let token = CancellationToken::new();
        let join = self
            .rt
            .spawn(Self::run_search(self.event_tx.clone(), token.clone(), input));

        self.searcher = Some((token, join));
    }

    fn stop_search(&mut self) {
        if let Some(mut thread) = self.searcher.take() {
            thread.0.cancel();
            self.rt.block_on(&mut thread.1).unwrap();
        }

        self.search = Search::default();
    }

    async fn run_search(event_tx: EventSender, token: CancellationToken, input: String) {
        select! {
            _ = Self::fetch_search(event_tx, input) => {},
            _ = token.cancelled() => return,
        };
    }

    async fn fetch_search(event_tx: EventSender, input: String) -> Result<(), Box<dyn Error>> {
        let client = Client::new(String::from(invidious::INSTANCE), MethodAsync::ReqwestAsync);
        let input = format!("q={input}");
        let items = client.search(Some(&input)).await?.items;

        event_tx.send(Event::Fetch(items.clone().into())).unwrap();

        for i in 0..items.len() {
            let event_tx = event_tx.clone();
            let _ = Self::fetch_thumbnail(event_tx, i, items[i].clone()).await;
        }

        Ok(())
    }

    async fn fetch_thumbnail(event_tx: EventSender, i: usize, item: SearchItem) -> Result<(), Box<dyn Error>> {
        let url = match item {
            Video { thumbnails, .. } => thumbnails.first().and_then(|t| Some(t.url.clone())),
            Channel { thumbnails, .. } => thumbnails.first().and_then(|t| Some(t.url.clone())),
            _ => None,
        };

        let response = match url {
            Some(url) => reqwest::get(url).await?,
            None => return Ok(()),
        };

        let thumbnail = ImageReader::new(Cursor::new(response.bytes().await?.as_ref()))
            .with_guessed_format()?
            .decode()?;

        event_tx.send(Event::Thumbnail(i, thumbnail)).unwrap();

        Ok(())
    }
}
