use crate::app::VideosList;

use std::error::Error;
use std::sync::{Arc, Mutex};

use invidious::reqwest::asynchronous::functions::search;
use invidious::reqwest::asynchronous::Client;
use invidious::structs::universal::Search;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::either::Either;
use tokio_util::sync::CancellationToken;

type SharedSearch = Arc<Mutex<VideosList>>;

pub struct Worker {
    search: SharedSearch,
    rt: Runtime,
    thread: Option<(CancellationToken, JoinHandle<()>)>,
}

impl Default for Worker {
    fn default() -> Self {
        Self {
            search: SharedSearch::default(),
            rt: Runtime::new().unwrap(),
            thread: None,
        }
    }
}

impl Worker {
    pub fn start(&mut self) {
        assert!(self.thread.is_none());

        let token = CancellationToken::new();
        let join = self.rt.spawn(Self::run(self.search.clone(), token.clone()));

        self.thread = Some((token, join));
    }

    pub fn stop(&mut self) {
        if let Some(mut thread) = self.thread.take() {
            thread.0.cancel();
            self.rt.block_on(&mut thread.1).unwrap();
        }

        *self.search.lock().unwrap() = VideosList::default();
    }

    async fn run(search: SharedSearch, token: CancellationToken) {
        let client = Client::new(String::from("https://vid.puffyan.us"));
        let fetch = client.search(Some("q=rust programming"));

        let result = select! {
            s = fetch => s,
            _ = token.cancelled() => return,
        };

        // Lock only when data is received
        *search.lock().unwrap() = match result {
            Ok(s) => VideosList::with_items(s),
            Err(_) => VideosList::default(),
        }
    }

    pub fn get_search(&self) -> VideosList {
        (*self.search
            .lock()
            .unwrap())
            .clone()
    }
}
