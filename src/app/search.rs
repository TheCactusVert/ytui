use image::DynamicImage;
use invidious::hidden::SearchItem::{self, *};
use ratatui::widgets::{ListItem, ListState};

#[derive(Default)]
pub struct Search {
    items: Vec<(SearchItem, Option<DynamicImage>)>,
    selection: ListState,
}

impl From<Vec<SearchItem>> for Search {
    fn from(items: Vec<SearchItem>) -> Self {
        Self {
            items: items.into_iter().map(|i| (i, None)).collect(),
            selection: ListState::default(),
        }
    }
}

impl Search {
    pub fn get_list_split<'a>(&'a mut self) -> (Vec<ListItem<'a>>, &'a mut ListState) {
        (
            self.items
                .iter()
                .map(|item| {
                    ListItem::new(match &item.0 {
                        Video { title, .. } => title.as_str(),
                        Playlist { title, .. } => title.as_str(),
                        Channel { name, .. } => name.as_str(),
                        Unknown(_) => "Error",
                    })
                })
                .collect(),
            &mut self.selection,
        )
    }

    pub fn next_video(&mut self) {
        if self.items.len() != 0 {
            let i = match self.selection.selected() {
                Some(i) if i == self.items.len() - 1 => self.items.len() - 1,
                Some(mut i) => {
                    i += 1;
                    i %= self.items.len();
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
        if self.items.len() != 0 {
            let i = match self.selection.selected() {
                Some(i) if i == 0 => 0,
                Some(mut i) => {
                    i -= 1;
                    i %= self.items.len();
                    i
                }
                None => self.items.len() - 1,
            };
            self.selection.select(Some(i));
        } else {
            self.selection.select(None);
        }
    }

    pub fn selected_item(&self) -> Option<&(SearchItem, Option<DynamicImage>)> {
        self.selection.selected().and_then(|i| Some(&self.items[i]))
    }

    pub fn set_thumbnail(&mut self, i: usize, image: DynamicImage) {
        self.items[i].1 = Some(image);
    }
}
