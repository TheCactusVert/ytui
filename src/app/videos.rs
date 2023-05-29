use invidious::structs::universal::Search;
use ratatui::widgets::ListState;

#[derive(Clone)]
pub struct VideosList {
    pub state: ListState,
    pub search: Search,
}

impl Default for VideosList {
    fn default() -> VideosList {
        VideosList {
            state: ListState::default(),
            search: Search { items: Vec::new() },
        }
    }
}

impl VideosList {
    pub fn with_items(search: Search) -> VideosList {
        VideosList {
            state: ListState::default(),
            search,
        }
    }

    pub fn next(&mut self) {
        if self.search.items.len() != 0 {
            let i = match self.state.selected() {
                Some(mut i) => {
                    i += 1;
                    i %= self.search.items.len();
                    i
                }
                None => 0,
            };
            self.state.select(Some(i));
        } else {
            self.state.select(None);
        }
    }

    pub fn previous(&mut self) {
        if self.search.items.len() != 0 {
            let i = match self.state.selected() {
                Some(mut i) => {
                    i -= 1;
                    i %= self.search.items.len();
                    i
                }
                None => 0,
            };
            self.state.select(Some(i));
        } else {
            self.state.select(None);
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
