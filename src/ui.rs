use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap, Chart, Dataset, Axis},
    text::Span,  // Add this import
    Frame,
};
use crate::state::AppState;
use crate::gdp_reader::GDPData;
// Remove the unused import: use crate::data::GeoLevel;

pub fn draw<'a>(f: &mut Frame<'a>, state: &mut AppState) {
    // Check if we should display the GDP chart
    if state.gdp_chart_active && state.all_gdp_data.is_some() {
        draw_gdp_chart(f, state);
        return;
    }
    
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
        .map(|(year, value)| format!("PKB ({}):\n{}\nTab: wykres", year, GDPData::format_gdp_value(*value)))
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

fn draw_gdp_chart<'a>(f: &mut Frame<'a>, state: &AppState) {
    // Get country name and all GDP data
    let country_name = &state.list_items[state.selected];
    let all_gdp_data = state.all_gdp_data.as_ref().unwrap();
    
    // Convert GDP data to chart data points
    let mut data_points: Vec<(f64, f64)> = all_gdp_data
        .iter()
        .filter_map(|(year, value)| {
            year.parse::<f64>().ok().map(|y| (y, *value))
        })
        .collect();
    
    // Sort data points by year
    data_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    
    // Find min and max values for y-axis
    let min_year = data_points.first().map(|(y, _)| *y).unwrap_or(1960.0);
    let max_year = data_points.last().map(|(y, _)| *y).unwrap_or(2024.0);
    
    let max_gdp = data_points.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let y_max = (max_gdp * 1.1).ceil(); // Add 10% margin
    
    // Billion markers for y-axis
    let y_labels = vec![
        format!("0"),
        format!("{:.1}B", y_max / 4e9),
        format!("{:.1}B", y_max / 2e9),
        format!("{:.1}B", y_max * 3.0 / 4e9),
        format!("{:.1}B", y_max / 1e9),
    ];
    
    // Create x-axis labels (years)
    let year_span = max_year - min_year;
    let step = (year_span / 6.0).ceil();
    let mut x_labels = Vec::new();
    
    for i in 0..=6 {
        let year = min_year + step * (i as f64);
        x_labels.push(format!("{}", year as i32));
    }
    
    // Create the chart
    let dataset = Dataset::default()
        .name(format!("GDP of {}", country_name))
        .marker(symbols::Marker::Bar)
        .style(Style::default().fg(Color::Green))
        .data(&data_points);
    
    let chart = Chart::new(vec![dataset])
        .block(Block::default()
            .title(format!("GDP History for {} (Press Tab to return)", country_name))
            .borders(Borders::ALL))
        .x_axis(Axis::default()
            .title("Year")
            .style(Style::default().fg(Color::Gray))
            .bounds([min_year, max_year])
            .labels(x_labels.iter().cloned().map(|s| Span::from(s)).collect::<Vec<Span>>()))
        .y_axis(Axis::default()
            .title("GDP (USD)")
            .style(Style::default().fg(Color::Gray))
            .bounds([0.0, y_max])
            .labels(y_labels.iter().cloned().map(|s| Span::from(s)).collect::<Vec<Span>>()));
    
    f.render_widget(chart, f.area());
}
