use crossterm::event::KeyCode;
use crate::{
    data::{CountryInfo, DataCache, GeoLevel},
    map_draw::MapView,
};

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
    pub active_panel: Panel,
}

impl AppState {
    const HELP_TEXT: &'static str = "\
↑/↓: ruch w liście
Enter: zagłębienie (świat→kontynent→kraj)
Esc / Backspace: wstecz
q: wyjście";

    pub fn new(data_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut cache = DataCache::new(data_dir)?;
        // 1) lista kontynentów
        let continents = cache.load_list(GeoLevel::World, "world")?;
        // 2) mapa świata
        let raw = cache.load_geojson(&GeoLevel::World, "world")?;
        let view = MapView::new(raw)?;
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
            active_panel: Panel::Left,
        })
    }

    /// Zwraca true, jeśli trzeba wyjść
    pub fn handle_input(&mut self, key: KeyCode) -> bool {
        use KeyCode::*;
        match key {
            Char('q') => return true,
            Tab => {
                self.active_panel = match self.active_panel {
                    Panel::Left   => Panel::Center,
                    Panel::Center => Panel::Right,
                    Panel::Right  => Panel::Left,
                };
            }
            Up => if self.selected > 0 { self.selected -= 1 },
            Down => if self.selected + 1 < self.list_items.len() { self.selected += 1 },
            Enter => {
                let choice = self.list_items[self.selected].clone();
                match self.level {
                    GeoLevel::World => {
                        // świat → kontynent
                        if let Ok(items) = self.cache.load_list(GeoLevel::Continent, &choice) {
                            self.history.push((GeoLevel::World, choice.clone()));
                            self.level = GeoLevel::Continent;
                            self.list_items = items;
                            self.selected = 0;
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Continent, &choice) {
                                if let Ok(view) = MapView::new(raw) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("{} – {} obiektów\n\n{}", choice, count, Self::HELP_TEXT);
                                    self.country_info = None;
                                }
                            }
                        }
                    }
                    GeoLevel::Continent => {
                        // kontynent → kraj
                        // pobierz nazwę kontynentu z ostatniego wpisu historii
                        if let Some((_, continent)) = self.history.last() {
                            self.history.push((GeoLevel::Continent, continent.clone()));
                            self.level = GeoLevel::Country;
                            self.list_items = vec![choice.clone()];
                            self.selected = 0;
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Country, &choice) {
                                if let Ok(view) = MapView::new(raw) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.country_info = self.cache.load_country_info(&choice).cloned();
                                    self.info = format!("{} – {} obiektów\n\n{}", choice, count, Self::HELP_TEXT);
                                }
                            }
                        }
                    }
                    GeoLevel::Country => {
                        // Enter nic nie robi
                    }
                }
            }
            Backspace | Esc => {
                if let Some((prev_lvl, prev_key)) = self.history.pop() {
                    self.country_info = None;
                    match prev_lvl {
                        GeoLevel::World => {
                            // wracamy do świata
                            if let Ok(cts) = self.cache.load_list(GeoLevel::World, "world") {
                                self.level = GeoLevel::World;
                                self.list_items = cts.clone();
                                self.selected = cts.iter().position(|s| s == &prev_key).unwrap_or(0);
                            }
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::World, "world") {
                                if let Ok(view) = MapView::new(raw) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("Świat – {} obiektów\n\n{}", count, Self::HELP_TEXT);
                                }
                            }
                        }
                        GeoLevel::Continent => {
                            // wracamy do widoku kontynentu
                            self.level = GeoLevel::Continent;
                            if let Ok(items) = self.cache.load_list(GeoLevel::Continent, &prev_key) {
                                self.list_items = items.clone();
                                self.selected = items.iter().position(|s| s == &prev_key).unwrap_or(0);
                            }
                            if let Ok(raw) = self.cache.load_geojson(&GeoLevel::Continent, &prev_key) {
                                if let Ok(view) = MapView::new(raw) {
                                    let count = view.feature_count();
                                    self.map = Some(view);
                                    self.info = format!("{} – {} obiektów\n\n{}", prev_key, count, Self::HELP_TEXT);
                                }
                            }
                        }
                        GeoLevel::Country => {
                            // nie powinno się zdarzyć
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }
}
