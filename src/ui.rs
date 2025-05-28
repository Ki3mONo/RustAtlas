use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use crate::state::AppState;

pub fn draw<'a>(f: &mut Frame<'a>, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)].as_ref())
        .split(f.area());

    // Lewy panel: lista
    let items: Vec<ListItem> = state.list_items.iter().map(|i| ListItem::new(i.clone())).collect();
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Wybór"))
        .highlight_symbol(">> ")
        .highlight_style(Style::default().fg(Color::Red));
    f.render_stateful_widget(list, chunks[0], &mut list_state);

    // Środek: mapa
    if let Some(map) = &state.map {
        let name = &state.list_items[state.selected];
        map.render(f, chunks[1], name, Some(name.as_str()));
    } else {
        let txt = Paragraph::new("Wybierz jednostkę, aby zobaczyć mapę")
            .block(Block::default().borders(Borders::ALL).title("Mapa"));
        f.render_widget(txt, chunks[1]);
    }

    // Prawy panel: informacje
    let info_block = if let Some(ci) = &state.country_info {
        let txt = format!(
            "{}\nStolica: {}\nPowierzchnia: {:.0} km²\nLudność: {}\nWaluta: {}",
            ci.name, ci.capital, ci.area, ci.population, ci.currency
        );
        Paragraph::new(txt)
    } else {
        Paragraph::new(state.info.as_str())
    };
    f.render_widget(
        info_block.block(Block::default().borders(Borders::ALL).title("Informacje")),
        chunks[2],
    );
}
