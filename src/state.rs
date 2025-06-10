use crossterm::event::KeyCode;
use crate::{
    data::{CountryInfo, DataCache, GeoLevel},
    map_draw::MapView,
    gdp_reader::GDPData,
};
use std::path::Path;
use std::collections::HashMap;

#[derive(PartialEq)]
pub enum Panel { Left, Center, Right }

pub struct AppState {
    pub cache: DataCache,
    pub level: GeoLevel,
    pub list_items: Vec<String>,
    pub selected: usize,
    pub history: Vec<(GeoLevel, String)>,
    pub map: Option<MapView>,
    pub info: String,
    pub country_info: Option<CountryInfo>,
    pub fun_fact: Option<String>,
    pub active_panel: Panel,
    pub gdp_data: Option<GDPData>,
    pub current_gdp: Option<(String, f64)>,
    pub gdp_chart_active: bool,
    pub all_gdp_data: Option<HashMap<String, f64>>, // Year -> Value
}

impl AppState {
    const HELP_TEXT: &'static str = "\
↑/↓: ruch w liście
Enter: zagłębienie (świat→kontynent→kraj)
Esc / Backspace: wstecz
q: wyjście";

    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self, Box<dyn std::error::Error>> {
        let base_dir = dir.as_ref();
        let mut cache = DataCache::new(base_dir)?;
        
        // Load GDP data if available
        let gdp_path = base_dir.join("dataPKB/pkb.csv");
        
        let gdp_data = match GDPData::new(&gdp_path) {
            Ok(data) => Some(data),
            Err(_) => None
        };
        
        let continents = cache.load_list(GeoLevel::World, "world")?;
        let raw = cache.load_geojson(&GeoLevel::World, "world")?;
        let view = MapView::new(raw, &mut cache)?;
        let count = view.feature_count();
        let info = format!("Świat – {} obiektów\n\n{}", count, Self::HELP_TEXT);

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

    fn update_gdp(&mut self, country_name: &str) {
        if let Some(gdp_data) = &self.gdp_data {
            self.current_gdp = gdp_data.get_latest_gdp(country_name);
        }
    }

    pub fn handle_input(&mut self, key: KeyCode) -> bool {
        use KeyCode::*;
        match key {
            Char('q') => return true,
            Tab => {
                // If we're viewing a country and have GDP data, show the chart
                if self.level == GeoLevel::Country && self.current_gdp.is_some() {
                    // Toggle GDP chart mode
                    self.gdp_chart_active = !self.gdp_chart_active;
                    
                    // If turning on chart mode, fetch all GDP data for the country
                    if self.gdp_chart_active && self.gdp_data.is_some() {
                        let country_name = &self.list_items[self.selected];
                        self.all_gdp_data = self.gdp_data.as_ref()
                            .and_then(|data| data.get_all_gdp_data(country_name));
                    }
                } else {
                    // Original panel cycling behavior
                    self.active_panel = match self.active_panel {
                        Panel::Left   => Panel::Center,
                        Panel::Center => Panel::Right,
                        Panel::Right  => Panel::Left,
                    };
                }
            }
            Up => if self.selected > 0 { self.selected -= 1 },
            Down => if self.selected + 1 < self.list_items.len() { self.selected += 1 },
            Enter => {
                // Skip if in chart mode
                if self.gdp_chart_active {
                    return false;
                }
                let choice = self.list_items[self.selected].clone();
                match self.level {
                    GeoLevel::World => {
                        if let Ok(items) = self.cache.load_list(GeoLevel::Continent, &choice) {
                            self.history.push((GeoLevel::World, choice.clone()));
                            self.level = GeoLevel::Continent;
                            self.list_items = items;
                            self.selected = 0;
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Continent, &choice) {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("{} – {} obiektów\n\n{}", choice, count, Self::HELP_TEXT);
                                    self.country_info = None;
                                    self.fun_fact = None;
                                }
                            }
                        }
                    }
                    GeoLevel::Continent => {
                        if let Some((_, continent)) = self.history.last() {
                            self.history.push((GeoLevel::Continent, continent.clone()));
                            self.level = GeoLevel::Country;
                            self.list_items = vec![choice.clone()];
                            self.selected = 0;
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Country, &choice) {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.country_info = self.cache.load_country_info(&choice).cloned();
                                    self.fun_fact   = self.cache.random_funfact(&choice);
                                    self.info       = format!("{} – {} obiektów\n\n{}", choice, count, Self::HELP_TEXT);
                                    self.update_gdp(&choice);
                                }
                            }
                        }
                    }
                    GeoLevel::Country => {}
                }
            }
            Backspace | Esc => {
                // Skip if in chart mode - only Tab works to exit the chart
                if self.gdp_chart_active {
                    return false;
                }
                if let Some((prev_lvl, prev_key)) = self.history.pop() {
                    self.country_info = None;
                    self.fun_fact = None;
                    self.current_gdp = None;  // Add this line to clear GDP data
                    
                    match prev_lvl {
                        GeoLevel::World => {
                            if let Ok(cts) = self.cache.load_list(GeoLevel::World, "world") {
                                self.level = GeoLevel::World;
                                self.list_items = cts.clone();
                                self.selected = cts.iter().position(|s| s == &prev_key).unwrap_or(0);
                            }
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::World, "world") {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("Świat – {} obiektów\n\n{}", count, Self::HELP_TEXT);
                                }
                            }
                        }
                        GeoLevel::Continent => {
                            self.level = GeoLevel::Continent;
                            if let Ok(items) = self.cache.load_list(GeoLevel::Continent, &prev_key) {
                                self.list_items = items.clone();
                                self.selected = items.iter().position(|s| s == &prev_key).unwrap_or(0);
                            }
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Continent, &prev_key) {
                                if let Ok(view) = MapView::new(raw, &mut self.cache) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("{} – {} obiektów\n\n{}", prev_key, count, Self::HELP_TEXT);
                                }
                            }
                        }
                        GeoLevel::Country => {}
                    }
                }
            }
            _ => {}
        }
        false
    }
}
