use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, List, ListItem, ListState, Paragraph, Wrap},
    Frame, text::Span,
};
use crate::state::AppState;
use crate::gdp_reader::GDPData;

/// Main draw function: either shows GDP chart or the three-panel view
pub fn draw<'a>(f: &mut Frame<'a>, state: &mut AppState) {
    // If detailed GDP chart is active, render it and return early
    if state.gdp_chart_active && state.all_gdp_data.is_some() {
        draw_gdp_chart(f, state);
        return;
    }

    // Split the terminal horizontally into left, center, and right panels
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // selection list
            Constraint::Percentage(60), // map view
            Constraint::Percentage(20), // info and charts
        ].as_ref())
        .split(f.area());

    // Left panel: show the selection list with highlight
    let items: Vec<ListItem> = state.list_items
        .iter()
        .map(|i| ListItem::new(i.clone()))
        .collect();
    let mut ls = ListState::default();
    ls.select(Some(state.selected));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Selection"))
        .highlight_symbol(">> ")
        .highlight_style(Style::default().fg(Color::Red));
    f.render_stateful_widget(list, chunks[0], &mut ls);

    // Center panel: render the map if available, otherwise placeholder text
    if let Some(map) = &state.map {
        let name = &state.list_items[state.selected];
        map.render(f, chunks[1], name, Some(name.as_str()));
    } else {
        let placeholder = Paragraph::new("Select an item to view the map")
            .block(Block::default().borders(Borders::ALL).title("Map"))
            .wrap(Wrap { trim: true });
        f.render_widget(placeholder, chunks[1]);
    }

    // Right panel: vertical split for info, GDP summary, and fun fact
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // country info or status
            Constraint::Percentage(30), // GDP summary
            Constraint::Percentage(30), // fun fact
        ].as_ref())
        .split(chunks[2]);

    // Info block: show country details or default help text
    let info_text = if let Some(ci) = &state.country_info {
        format!(
            "{}\nCapital: {}\nArea: {:.0} km²\nPopulation: {}\nCurrency: {}",
            ci.name, ci.capital, ci.area, ci.population, ci.currency
        )
    } else {
        state.info.clone()
    };
    let info = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title("Info"))
        .wrap(Wrap { trim: true });
    f.render_widget(info, right_chunks[0]);

    // GDP summary block: latest GDP value with prompt to view chart
    let gdp_text = state.current_gdp.as_ref()
        .map(|(year, value)| {
            format!(
                "GDP ({}):\n{}\nPress Tab to view chart!",
                year,
                GDPData::format_gdp_value(*value)
            )
        })
        .unwrap_or_else(|| "Select a country to view GDP data".to_string());
    let gdp = Paragraph::new(gdp_text)
        .block(Block::default().borders(Borders::ALL).title("GDP"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(gdp, right_chunks[1]);

    // Fun fact block: random fact or prompt to select a country
    let fact_text = state.fun_fact
        .as_deref()
        .unwrap_or("Select a country to view a fun fact");
    let fact = Paragraph::new(fact_text)
        .block(Block::default().borders(Borders::ALL).title("Did you know? (PL Version)"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(fact, right_chunks[2]);
}

/// Draw the detailed GDP history chart for the selected country
fn draw_gdp_chart<'a>(f: &mut Frame<'a>, state: &AppState) {
    let country = &state.list_items[state.selected];
    let all = state.all_gdp_data.as_ref().unwrap();

    // Prepare sorted (year, value) points for the chart
    let mut pts: Vec<(f64, f64)> = all
        .iter()
        .filter_map(|(yr_str, &val)| yr_str.parse::<f64>().ok().map(|yr| (yr, val)))
        .collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Determine axis bounds
    let min_year = pts.first().map(|&(y, _)| y).unwrap_or(1960.0);
    let max_year = pts.last().map(|&(y, _)| y).unwrap_or(2024.0);
    let max_gdp = pts.iter().map(|&(_, v)| v).fold(0.0, f64::max);
    let y_max = (max_gdp * 1.1).ceil();

    // Labels for axes
    let y_labels = vec![
        "0".to_string(),
        format!("{:.1}B", y_max / 4e9),
        format!("{:.1}B", y_max / 2e9),
        format!("{:.1}B", y_max * 3.0 / 4e9),
        format!("{:.1}B", y_max / 1e9),
    ];
    let span = max_year - min_year;
    let step = (span / 6.0).ceil();
    let x_labels: Vec<Span> = (0..=6)
        .map(|i| Span::from(((min_year + step * i as f64) as i32).to_string()))
        .collect();

    // Dataset for the chart
    let ds = Dataset::default()
        .name(format!("GDP {}", country))
        .marker(symbols::Marker::Bar)
        .style(Style::default().fg(Color::Green))
        .data(&pts);

    let chart = Chart::new(vec![ds])
        .block(
            Block::default()
                .title(format!(
                    "{} GDP History (Press Tab to return to map view)",
                    country
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Year")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_year, max_year])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("GDP (USD)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, y_max])
                .labels(y_labels.into_iter().map(Span::from).collect::<Vec<Span>>()),
        );

    // Render the chart to fill the terminal
    f.render_widget(chart, f.area());
}
