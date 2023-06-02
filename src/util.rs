use crate::app::search::Search;
use ratatui::style::Style;
use ratatui::widgets::ListItem;

pub fn search_to_list_items<'a>(search: &'a Vec<Search>) -> Vec<ListItem<'a>> {
    search
        .iter()
        .map(|v| {
            ListItem::new(match v {
                Search::Video { title, .. } => title.as_str(),
                Search::Playlist { title, .. } => title.as_str(),
                Search::Channel { name, .. } => name.as_str(),
                Search::Unknown(_) => "Error",
            })
            .style(Style::default())
        })
        .collect()
}
