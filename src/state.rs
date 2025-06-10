use crossterm::event::KeyCode;
use crate::{
    data::{CountryInfo, DataCache, GeoLevel},
    map_draw::MapView,
    gdp_reader::GDPData,
};
use std::{path::Path, collections::HashMap};

#[derive(PartialEq)]
/// UI panel focus states
pub enum Panel { Left, Center, Right }

pub struct AppState {
    pub cache: DataCache,                  // data loader and cache
    pub level: GeoLevel,                   // current geographic level
    pub list_items: Vec<String>,           // items in the selection list
    pub selected: usize,                   // index of the selected item
    pub history: Vec<(GeoLevel, String)>,  // navigation history stack
    pub map: Option<MapView>,              // current map view
    pub info: String,                      // status and help text
    pub country_info: Option<CountryInfo>, // metadata for the selected country
    pub fun_fact: Option<String>,          // random fun fact for a country
    pub active_panel: Panel,               // currently focused panel
    pub gdp_data: Option<GDPData>,         // optional GDP dataset
    pub current_gdp: Option<(String, f64)>,// latest GDP (year, value)
    pub gdp_chart_active: bool,            // whether detailed GDP chart is active
    pub all_gdp_data: Option<HashMap<String, f64>>, // full GDP history for chart
}

impl AppState {
    // Help instructions shown in the info panel
    const HELP_TEXT: &'static str = "\
↑/↓: move selection
Enter: drill down (world → continent → country)
Esc / Backspace: go back
q: quit";

    /// Initialize application state: load data, map, and help text
    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self, Box<dyn std::error::Error>> {
        let base = dir.as_ref();
        let mut cache = DataCache::new(base)?;

        // Attempt to load GDP dataset
        let gdp_data = GDPData::new(&base.join("dataPKB/pkb.csv")).ok();

        // Load world-level list and map view
        let continents = cache.load_list(GeoLevel::World, "world")?;
        let raw = cache.load_geojson(&GeoLevel::World, "world")?;
        let view = MapView::new(raw, &mut cache)?;
        let count = view.feature_count();
        let info = format!("World – {} features\n\n{}", count, Self::HELP_TEXT);

        Ok(Self {
            cache,
            level: GeoLevel::World,
            list_items: continents,
            selected: 0,
            history: Vec::new(),
            map: Some(view),
            info,
            country_info: None,
            fun_fact: None,
            active_panel: Panel::Left,
            gdp_data,
            current_gdp: None,
            gdp_chart_active: false,
            all_gdp_data: None,
        })
    }

    /// Update `current_gdp` to the latest available for a given country
    fn update_gdp(&mut self, country_name: &str) {
        if let Some(data) = &self.gdp_data {
            self.current_gdp = data
                .get_latest_gdp(country_name)
                .map(|(year, val)| (year.to_string(), val));
        } else {
            self.current_gdp = None;
        }
    }

    /// Handle key events; return true to exit application
    pub fn handle_input(&mut self, key: KeyCode) -> bool {
        use KeyCode::*;
        match key {
            Char('q') => return true, // quit application

            Tab => {
                // Toggle GDP chart or cycle panel focus
                if self.level == GeoLevel::Country && self.current_gdp.is_some() {
                    self.gdp_chart_active = !self.gdp_chart_active;
                    if self.gdp_chart_active {
                        // Load full GDP history for chart view
                        if let Some(data) = &self.gdp_data {
                            let country = &self.list_items[self.selected];
                            self.all_gdp_data = data
                                .get_all_gdp_data(country)
                                .map(|btree| btree.iter()
                                    .map(|(&y, &v)| (y.to_string(), v))
                                    .collect());
                        }
                    } else {
                        // Clear detailed GDP history on exit
                        self.all_gdp_data = None;
                    }
                } else {
                    // Cycle focus between left, center, and right panels
                    self.active_panel = match self.active_panel {
                        Panel::Left => Panel::Center,
                        Panel::Center => Panel::Right,
                        Panel::Right => Panel::Left,
                    };
                }
            }

            Up => { if self.selected > 0 { self.selected -= 1; } }
            Down => { if self.selected + 1 < self.list_items.len() { self.selected += 1; } }

            Enter => {
                if self.gdp_chart_active { return false; }
                let choice = self.list_items[self.selected].clone();
                match self.level {
                    GeoLevel::World => {
                        // Drill down to continent level
                        if let Ok(items) = self.cache.load_list(GeoLevel::Continent, &choice) {
                            self.history.push((GeoLevel::World, choice.clone()));
                            self.level = GeoLevel::Continent;
                            self.list_items = items;
                            self.selected = 0;
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Continent, &choice) {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    let cnt = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("{} – {} features\n\n{}", choice, cnt, Self::HELP_TEXT);
                                }
                            }
                            self.country_info = None;
                            self.fun_fact = None;
                        }
                    }
                    GeoLevel::Continent => {
                        // Drill down to country level
                        if let Some((_, cont)) = self.history.last() {
                            self.history.push((GeoLevel::Continent, cont.clone()));
                            self.level = GeoLevel::Country;
                            self.list_items = vec![choice.clone()];
                            self.selected = 0;
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Country, &choice) {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    self.map = Some(view);
                                    self.country_info = self.cache.load_country_info(&choice).cloned();
                                    self.fun_fact = self.cache.random_funfact(&choice);
                                    self.info = format!("{} – 1 feature\n\n{}", choice, Self::HELP_TEXT);
                                    self.update_gdp(&choice);
                                }
                            }
                        }
                    }
                    GeoLevel::Country => {}
                }
            }

            Backspace | Esc => {
                if self.gdp_chart_active { return false; }
                if let Some((prev_lvl, prev_key)) = self.history.pop() {
                    // Reset country-specific data on back
                    self.country_info = None;
                    self.fun_fact = None;
                    self.current_gdp = None;
                    self.all_gdp_data = None;

                    // Navigate back to previous level
                    if prev_lvl == GeoLevel::World {
                        if let Ok(list) = self.cache.load_list(GeoLevel::World, "world") {
                            self.level = GeoLevel::World;
                            self.list_items = list;
                            self.selected = self.list_items.iter().position(|s| s == &prev_key).unwrap_or(0);
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::World, "world") {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    let cnt = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("World – {} features\n\n{}", cnt, Self::HELP_TEXT);
                                }
                            }
                        }
                    } else if prev_lvl == GeoLevel::Continent {
                        self.level = GeoLevel::Continent;
                        if let Ok(items) = self.cache.load_list(GeoLevel::Continent, &prev_key) {
                            self.list_items = items;
                            self.selected = self.list_items.iter().position(|s| s == &prev_key).unwrap_or(0);
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Continent, &prev_key) {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    let cnt = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("{} – {} features\n\n{}", prev_key, cnt, Self::HELP_TEXT);
                                }
                            }
                        }
                    }
                }
            }

            _ => {}
        }
        false
    }
}
