/// Provides map rendering view with geographic features and optional highlighting.
use geo::{Geometry, MultiPolygon, Polygon};
use geojson::GeoJson;
use std::{collections::{HashMap, HashSet}, error::Error};
use crate::data::DataCache;
use ratatui::widgets::canvas::{Canvas, Line};
use ratatui::{layout::Rect as TuiRect, Frame, style::Color};

/// Calculates the absolute area of a polygon via the shoelace formula.
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

pub struct MapView {
    items: Vec<(String, MultiPolygon<f64>)>,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    continents: HashMap<String, HashSet<String>>,
}

impl MapView {
    /// Initialize view from GeoJSON and load continent mappings.
    pub fn new(raw: GeoJson, data_cache: &mut DataCache) -> Result<Self, Box<dyn Error>> {
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

                    // Filter out small holes by area threshold
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

        // Determine spatial bounds of all features
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

        let continents = data_cache.load_continent_mappings().unwrap_or_default();
        Ok(Self { items, x_bounds: [minx, maxx], y_bounds: [miny, maxy], continents })
    }

    /// Returns number of geographic features loaded.
    pub fn feature_count(&self) -> usize {
        self.items.len()
    }

    /// Render all polygons, optionally highlighting a continent or country in red.
    pub fn render<'a>(
        &self,
        f: &mut Frame<'a>,
        area: TuiRect,
        title: &str,
        highlight: Option<&str>,
    ) {
        // Helper closure to draw a polygon path in a given color
        let draw_poly = |ctx: &mut ratatui::widgets::canvas::Context, poly: &Polygon<f64>, color: Color| {
            for window in poly.exterior().0.windows(2) {
                let a = window[0];
                let b = window[1];
                ctx.draw(&Line { x1: a.x, y1: a.y, x2: b.x, y2: b.y, color });
            }
            if let (Some(first), Some(last)) = (poly.exterior().0.first(), poly.exterior().0.last()) {
                ctx.draw(&Line { x1: last.x, y1: last.y, x2: first.x, y2: first.y, color });
            }
        };

        let canvas = Canvas::default()
            .block(ratatui::widgets::Block::default()
                .title(title)
                .borders(ratatui::widgets::Borders::ALL))
            .x_bounds(self.x_bounds)
            .y_bounds(self.y_bounds)
            .paint(|ctx| {
                // Draw all features in white
                for (_, mp) in &self.items {
                    for poly in &mp.0 {
                        draw_poly(ctx, poly, Color::White);
                    }
                }

                // If highlighting, draw selected features in red
                if let Some(sel) = highlight {
                    let highlight_color = Color::Red;
                    // Check if it's a continent (multiple countries)
                    if let Some(countries) = self.continents.get(sel) {
                        for (name, mp) in &self.items {
                            if countries.contains(name) {
                                for poly in &mp.0 {
                                    draw_poly(ctx, poly, highlight_color);
                                }
                            }
                        }
                    } else {
                        // Single country highlight
                        for (name, mp) in &self.items {
                            if name == sel {
                                for poly in &mp.0 {
                                    draw_poly(ctx, poly, highlight_color);
                                }
                            }
                        }
                    }
                }
            });
        f.render_widget(canvas, area);
    }
}
