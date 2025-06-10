use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use crate::state::AppState;
use crate::gdp_reader::GDPData;
use crate::data::GeoLevel; // Add this import

pub fn draw<'a>(f: &mut Frame<'a>, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ].as_ref())
        .split(f.area());

    // Lewy panel: lista
    let items: Vec<ListItem> = state.list_items
        .iter()
        .map(|i| ListItem::new(i.clone()))
        .collect();
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
            .block(Block::default().borders(Borders::ALL).title("Mapa"))
            .wrap(Wrap { trim: true });
        f.render_widget(txt, chunks[1]);
    }

    // Prawy panel: Informacje + PKB + Czy wiesz, że...
    let right = chunks[2];
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ].as_ref())
        .split(right);

    // — Informacje
    let info_text = if let Some(ci) = &state.country_info {
        format!(
            "{}\nStolica: {}\nPowierzchnia: {:.0} km²\nLudność: {}\nWaluta: {}",
            ci.name, ci.capital, ci.area, ci.population, ci.currency
        )
    } else {
        state.info.clone()
    };
    let info_paragraph = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title("Informacje"))
        .wrap(Wrap { trim: true });
    f.render_widget(info_paragraph, right_chunks[0]);

    // -- PKB
    let gdp_text = state.current_gdp.as_ref()
        .map(|(year, value)| format!("PKB ({}):\n{}", year, GDPData::format_gdp_value(*value)))
        .unwrap_or_else(|| "Wybierz kraj, aby zobaczyć dane PKB".to_string());
    
    let gdp_paragraph = Paragraph::new(gdp_text)
        .block(Block::default().borders(Borders::ALL).title("PKB"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(gdp_paragraph, right_chunks[1]);

    // — Czy wiesz, że...
    let fact_txt = state.fun_fact
        .as_deref()
        .unwrap_or("Wybierz kraj, aby zobaczyć ciekawostkę");
    let fact_paragraph = Paragraph::new(fact_txt)
        .block(Block::default().borders(Borders::ALL).title("Czy wiesz, że..."))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(fact_paragraph, right_chunks[2]);
}
