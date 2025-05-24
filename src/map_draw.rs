use geojson::GeoJson;
use std::error::Error;
use geo::{Geometry, MultiPolygon, Polygon};
use ratatui::widgets::canvas::{Canvas, Line};
use ratatui::layout::Rect as TuiRect;
use ratatui::{Frame, backend::Backend, style::Color};

/// Liczy pole (w przybliżeniu płaskim) wielokąta wzorem shoelace’a.
fn poly_area(poly: &Polygon<f64>) -> f64 {
    let coords = &poly.exterior().0;
    let mut sum = 0.0;
    for window in coords.windows(2) {
        let a = window[0];
        let b = window[1];
        sum += a.x * b.y - b.x * a.y;
    }
    (sum * 0.5).abs()
}

/// Przygotowanie geometrii i rysowanie mapy
pub struct MapView {
    items: Vec<(String, MultiPolygon<f64>)>,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
}

impl MapView {
    pub fn new(raw: GeoJson) -> Result<Self, Box<dyn Error>> {
        let mut items = Vec::new();

        if let GeoJson::FeatureCollection(fc) = raw {
            for feature in fc.features {
                let name = feature
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("ADMIN").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();

                if let Some(gj) = feature.geometry {
                    let geom: Geometry<f64> = gj.value.try_into()?;
                    let mut mp = match geom {
                        Geometry::Polygon(p) => p.into(),
                        Geometry::MultiPolygon(m) => m,
                        _ => continue,
                    };

                    // Filtrujemy drobne fragmenty, jeśli jest ich wiele
                    if mp.0.len() > 1 {
                        let orig: Vec<Polygon<f64>> = mp.0.clone();
                        let areas: Vec<f64> = orig.iter().map(poly_area).collect();
                        let max_area = areas.iter().cloned().fold(0./0., f64::max);
                        let threshold = max_area * 0.20;
                        let filtered: Vec<Polygon<f64>> = orig.into_iter()
                            .zip(areas.into_iter())
                            .filter(|(_, area)| *area >= threshold)
                            .map(|(poly, _)| poly)
                            .collect();
                        if !filtered.is_empty() {
                            mp = MultiPolygon(filtered);
                        }
                    }

                    items.push((name, mp));
                }
            }
        }

        // Ustal zakresy współrzędnych
        let (mut minx, mut miny, mut maxx, mut maxy) =
            (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (_, mp) in &items {
            for poly in &mp.0 {
                for coord in poly.exterior().0.iter()
                    .chain(poly.interiors().iter().flat_map(|r| r.0.iter()))
                {
                    minx = minx.min(coord.x);
                    miny = miny.min(coord.y);
                    maxx = maxx.max(coord.x);
                    maxy = maxy.max(coord.y);
                }
            }
        }

        Ok(Self { items, x_bounds: [minx, maxx], y_bounds: [miny, maxy] })
    }

    /// Liczba obiektów (np. krajów)
    pub fn feature_count(&self) -> usize {
        self.items.len()
    }

    /// Rysuje mapę, najpierw wszystkie granice, a potem podświetla jeden wybrany
    pub fn render<B: Backend>(
        &self,
        f: &mut Frame<B>,
        area: TuiRect,
        title: &str,
        highlight: Option<&str>,
    ) {
        let canvas = Canvas::default()
            .block(ratatui::widgets::Block::default().title(title).borders(ratatui::widgets::Borders::ALL))
            .x_bounds(self.x_bounds)
            .y_bounds(self.y_bounds)
            .paint(|ctx| {
                // 1) Rysujemy wszystkie granice w białym
                for (_, mp) in &self.items {
                    for poly in &mp.0 {
                        for window in poly.exterior().0.windows(2) {
                            let a = window[0];
                            let b = window[1];
                            ctx.draw(&Line { x1: a.x, y1: a.y, x2: b.x, y2: b.y, color: Color::White });
                        }
                        if let (Some(first), Some(last)) = (poly.exterior().0.first(), poly.exterior().0.last()) {
                            ctx.draw(&Line { x1: last.x, y1: last.y, x2: first.x, y2: first.y, color: Color::White });
                        }
                    }
                }

                // 2) Podświetlamy wybrany obiekt na czerwono
                if let Some(sel) = highlight {
                    for (name, mp) in &self.items {
                        if name == sel {
                            for poly in &mp.0 {
                                for window in poly.exterior().0.windows(2) {
                                    let a = window[0];
                                    let b = window[1];
                                    ctx.draw(&Line { x1: a.x, y1: a.y, x2: b.x, y2: b.y, color: Color::Red });
                                }
                                if let (Some(first), Some(last)) = (poly.exterior().0.first(), poly.exterior().0.last()) {
                                    ctx.draw(&Line { x1: last.x, y1: last.y, x2: first.x, y2: first.y, color: Color::Red });
                                }
                            }
                        }
                    }
                }
            });
        f.render_widget(canvas, area);
    }
}