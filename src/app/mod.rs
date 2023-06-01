mod ui;

use crate::util;
use crate::Event;
use crate::EventSender;
use ui::*;

use std::convert::AsRef;
use std::io::Cursor;
use std::process::Command;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::Pixel;
use invidious::reqwest::asynchronous::Client;
use invidious::structs::hidden::SearchItem::*;
use invidious::structs::universal::Search;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{
        canvas::{Canvas, Points},
        Block, Borders, List, ListState, Paragraph,
    },
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

        if let Some(i) = self.result_search_selection.selected() {
            match &result_search.items[i] {
                Video { title, author, .. } => self.ui_video(f, chunks_b[1], title, author),
                Playlist { title, author, .. } => self.ui_playlist(f, chunks_b[1], title, author),
                Channel { name, description, .. } => self.ui_channel(f, chunks_b[1], name, description),
                Unknown { .. } => self.ui_empty(f, chunks_b[1]),
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
            .constraints([Constraint::Percentage(50), Constraint::Min(1), Constraint::Min(1)].as_ref())
            .split(rect);
        
        // TODO should be thumbnail
        // TODO this shit is slow as fuck
        //self.render_image(f, chunks[0], include_bytes!("../../static/logo.png")).unwrap();

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
            .constraints([Constraint::Percentage(50), Constraint::Min(1), Constraint::Min(1)].as_ref())
            .split(rect);

        let title = Paragraph::new(title).style(STYLE_TITLE);
        f.render_widget(title, chunks[1]);

        let author = Paragraph::new(author).style(STYLE_AUTHOR);
        f.render_widget(author, chunks[2]);
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
            .constraints([Constraint::Percentage(50), Constraint::Min(1), Constraint::Min(1)].as_ref())
            .split(rect);

        let name = Paragraph::new(name).style(STYLE_TITLE);
        f.render_widget(name, chunks[1]);

        let description = Paragraph::new(description).style(STYLE_AUTHOR);
        f.render_widget(description, chunks[2]);
    }

    fn ui_empty<B: Backend>(&self, f: &mut Frame<B>, rect: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.get_border_style(State::Item));
        f.render_widget(block, rect);
    }

    fn render_image<B: Backend, T: AsRef<[u8]>>(
        &self,
        f: &mut Frame<B>,
        rect: Rect,
        data: T,
    ) -> Result<()> {
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()?
            .decode()?
            .resize_exact(rect.width.into(), rect.height.into(), FilterType::Nearest)
            .to_rgb8();

        assert!(rect.width as u32 == img.width());
        assert!(rect.height as u32 == img.height());

        let canvas = Canvas::default()
            .x_bounds([0.0, (img.width() - 1) as f64])
            .y_bounds([0.0, (img.height() - 1) as f64])
            .paint(|p| {
                for x in 0..img.width() {
                    for y in 0..img.height() {
                        let pixel = img.get_pixel(x, y);
                        let rgb = pixel.to_rgb();
                        p.draw(&Points {
                            coords: &[(x as f64, y as f64)],
                            color: Color::Rgb(rgb.0[0], rgb.0[1], rgb.0[2]),
                        })
                    }
                }
            });

        f.render_widget(canvas, rect);

        Ok(())
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
