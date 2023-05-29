use invidious::structs::hidden::SearchItem::{Channel, Playlist, Unknown, Video};
use invidious::structs::universal::Search;
use ratatui::style::Style;
use ratatui::widgets::ListItem;

pub fn search_to_list_items<'a>(search: &'a Search) -> Vec<ListItem<'a>> {
    search
        .items
        .iter()
        .map(|v| {
            ListItem::new(match v {
                Video { title, .. } => title.as_str(),
                Playlist { title, .. } => title.as_str(),
                Channel { name, .. } => name.as_str(),
                Unknown(_) => "Error",
            })
            .style(Style::default())
        })
        .collect()
}
