use invidious::structs::hidden::SearchItem::{self, *};
use invidious::structs::universal::Search as Items;
use ratatui::widgets::{ListItem, ListState};

pub struct Search {
    items: Items,
    selection: ListState,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            items: Items { items: Vec::new() },
            selection: ListState::default(),
        }
    }
}

impl Search {
    pub fn from_items(items: Items) -> Self {
        Self {
            items,
            selection: ListState::default(),
        }
    }

    pub fn get_list_split<'a>(&'a mut self) -> (Vec<ListItem<'a>>, &'a mut ListState) {
        (self.items
            .items
            .iter()
            .map(|item| {
                ListItem::new(match item {
                    Video { title, .. } => title.as_str(),
                    Playlist { title, .. } => title.as_str(),
                    Channel { name, .. } => name.as_str(),
                    Unknown(_) => "Error",
                })
            })
            .collect(),
        &mut self.selection)
    }

    pub fn next_video(&mut self) {
        if self.items.items.len() != 0 {
            let i = match self.selection.selected() {
                Some(i) if i == self.items.items.len() - 1 => self.items.items.len() - 1,
                Some(mut i) => {
                    i += 1;
                    i %= self.items.items.len();
                    i
                }
                None => 0,
            };
            self.selection.select(Some(i));
        } else {
            self.selection.select(None);
        }
    }

    pub fn previous_video(&mut self) {
        if self.items.items.len() != 0 {
            let i = match self.selection.selected() {
                Some(i) if i == 0 => 0,
                Some(mut i) => {
                    i -= 1;
                    i %= self.items.items.len();
                    i
                }
                None => self.items.items.len() - 1,
            };
            self.selection.select(Some(i));
        } else {
            self.selection.select(None);
        }
    }

    pub fn selected_item(&self) -> Option<&SearchItem> {
        self.selection.selected().and_then(|i| Some(&self.items.items[i]))
    }
}
