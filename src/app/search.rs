use image::DynamicImage;
use invidious::structs::hidden::{SearchItem, SearchItemTransition};

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
        thumbnail: Option<Thumbnail>,
    },
    Unknown(SearchItemTransition)
}

impl From<SearchItem> for Search {
    fn from(item: SearchItem) -> Self {
        match item {
            SearchItem::Video { title, author, thumbnails, .. } => Search::Video { title, author, thumbnail: thumbnails.first().and_then(|t| Some(Thumbnail::Link(t.url.clone()))) },
            SearchItem::Playlist { title, author, thumbnail, .. } => Search::Playlist { title, author, thumbnail: Some(Thumbnail::Link(thumbnail)) },
            SearchItem::Channel { name, thumbnails, .. } => Search::Channel { name, thumbnail: thumbnails.first().and_then(|t| Some(Thumbnail::Link(t.url.clone()))) },
            SearchItem::Unknown(s) => Search::Unknown(s),
        }
    }
}
