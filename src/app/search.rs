use image::DynamicImage;
use invidious::structs::hidden::{SearchItem, SearchItemTransition};
use ratatui::widgets::ListItem;

pub enum Thumbnail {
    Link(String),
    Data(DynamicImage),
}

pub enum Search {
    Video {
        title: String,
        author: String,
        thumbnail: Option<Thumbnail>,
    },
    Playlist {
        title: String,
        author: String,
        thumbnail: Option<Thumbnail>,
    },
    Channel {
        name: String,
        description: String,
        thumbnail: Option<Thumbnail>,
    },
    Unknown(SearchItemTransition),
}

impl<'a>  Search {
    pub fn into_list_item(&'a self) -> ListItem<'a> {
        ListItem::new(match self {
            Search::Video { title, .. } => title.as_str(),
            Search::Playlist { title, .. } => title.as_str(),
            Search::Channel { name, .. } => name.as_str(),
            Search::Unknown(_) => "Error",
        })
    }
}

impl From<SearchItem> for Search {
    fn from(item: SearchItem) -> Self {
        match item {
            SearchItem::Video {
                title,
                author,
                thumbnails,
                ..
            } => Search::Video {
                title,
                author,
                thumbnail: thumbnails
                    .first()
                    .and_then(|t| Some(Thumbnail::Link(t.url.clone()))),
            },
            SearchItem::Playlist {
                title,
                author,
                thumbnail,
                ..
            } => Search::Playlist {
                title,
                author,
                thumbnail: Some(Thumbnail::Link(thumbnail)),
            },
            SearchItem::Channel {
                name,
                description,
                thumbnails,
                ..
            } => Search::Channel {
                name,
                description,
                thumbnail: thumbnails
                    .first()
                    .and_then(|t| Some(Thumbnail::Link(t.url.clone()))),
            },
            SearchItem::Unknown(s) => Search::Unknown(s),
        }
    }
}
