mod app;
mod args;
mod util;

use app::App;
use args::Args;

use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyEvent, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub enum Event {
    FocusGained,
    FocusLost,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    Resize(u16, u16),
    Fetch,
}

type EventSender = Sender<Event>;
type EventReceiver = Receiver<Event>;
type EventChannel = (EventSender, EventReceiver);

fn main() -> Result<()> {
    let args = Args::parse();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx): EventChannel = channel();
    let tx_clone = tx.clone();
    thread::spawn(move || loop {
        let event = match crossterm::event::read() {
            Ok(event) => match event {
                crossterm::event::Event::FocusGained => Event::FocusGained,
                crossterm::event::Event::FocusLost => Event::FocusLost,
                crossterm::event::Event::Key(k) => Event::Key(k),
                crossterm::event::Event::Mouse(m) => Event::Mouse(m),
                crossterm::event::Event::Paste(p) => Event::Paste(p),
                crossterm::event::Event::Resize(w, h) => Event::Resize(w, h),
            },
            Err(e) => continue,
        };

        tx_clone.send(event).unwrap();
    });

    // create app and run it
    let mut app = App::new(tx.clone());

    while app.is_running() {
        terminal.draw(|f| app.ui(f));

        match rx.recv() {
            Ok(event) => match event {
                Event::Key(key) => app.handle_key_event(key),
                _ => {}
            },
            Err(e) => {}
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
