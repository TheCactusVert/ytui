use crate::app::search::Search;
use ratatui::style::Style;
use ratatui::widgets::ListItem;

pub fn search_to_list_items<'a>(search: &'a Vec<Search>) -> Vec<ListItem<'a>> {
    search
        .iter()
        .map(|v| {
            v.into_list_item().style(Style::default())
        })
        .collect()
}
