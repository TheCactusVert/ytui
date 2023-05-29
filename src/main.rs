mod app;
mod args;
mod util;

use app::App;
use args::Args;

use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

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
        let event = crossterm::event::read().unwrap();
        tx_clone.send(event).unwrap();
    });

    // create app and run it
    let mut app = App::default();

    let mut ret = Ok(());

    while app.is_running() {
        terminal.draw(|f| app.ui(f));
        
        match rx.recv() {
            Ok(event) => app.handle_event(event),
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

    ret
}
