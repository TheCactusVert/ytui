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
        let i = match self.state.selected() {
            Some(mut i) if self.search.items.len() != 0 => {
                i += 1;
                i %= self.search.items.len();
                i
            }
            _ => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(mut i) if self.search.items.len() != 0 => {
                i -= 1;
                i %= self.search.items.len();
                i
            }
            _ => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
