use serde::Deserialize;
use serde_json::from_slice;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use geojson::GeoJson;

/// Poziomy: świat → kontynent → kraj
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeoLevel {
    World,
    Continent,
    Country,
}

/// Dane o kraju ładowane z country_info.json
#[derive(Clone, Debug, Deserialize)]
pub struct CountryInfo {
    pub name: String,
    pub capital: String,
    pub area: f64,
    pub population: u64,
    pub currency: String,
}

/// Proste ładowanie list (.json), geojson i danych krajów
pub struct DataCache {
    base: PathBuf,
    index: BTreeMap<(GeoLevel, String), Vec<String>>,
    country_info: Option<BTreeMap<String, CountryInfo>>,
}

impl DataCache {
    pub fn new<P: AsRef<Path>>(base: P) -> Result<Self, Box<dyn std::error::Error>> {
        let base = base.as_ref().to_path_buf();
        fs::create_dir_all(&base)?;
        // Spróbuj wczytać country_info.json
        let country_info = fs::read(base.join("country_info.json"))
            .ok()
            .and_then(|b| from_slice::<BTreeMap<String, CountryInfo>>(&b).ok());
        Ok(Self { base, index: BTreeMap::new(), country_info })
    }

    /// Wczytaj listę `<prefix>_<key>.json`
    pub fn load_list(&mut self, level: GeoLevel, key: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let skey = key.to_lowercase().replace(' ', "_").replace('(', "").replace(')', "");
        let filename = format!("{}_{}.json", match level {
            GeoLevel::World     => "continent",
            GeoLevel::Continent => "country",
            GeoLevel::Country   => "country",
        }, skey);
        let data = fs::read(self.base.join(&filename))?;
        let list: Vec<String> = from_slice(&data)?;
        self.index.insert((level, key.to_string()), list.clone());
        Ok(list)
    }

    /// Wczytaj `<prefix>_<key>.geojson`
    pub fn load_geojson(&self, level: &GeoLevel, key: &str) -> Result<GeoJson, Box<dyn std::error::Error>> {
        let skey = key.to_lowercase().replace(' ', "_").replace('(', "").replace(')', "");
        let prefix = match level {
            GeoLevel::World     => "continent",
            GeoLevel::Continent => "country",
            GeoLevel::Country   => "country",
        };
        let filename = format!("{}_{}.geojson", prefix, skey);
        let txt = fs::read_to_string(self.base.join(&filename))?;
        Ok(GeoJson::from_str(&txt)?)
    }

    /// Zwraca dane o kraju po jego kluczu (snake_case)
    pub fn load_country_info(&self, key: &str) -> Option<&CountryInfo> {
        let skey = key.to_lowercase().replace(' ', "_").replace('(', "").replace(')', "");
        self.country_info.as_ref()?.get(&skey)
    }
}