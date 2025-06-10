use serde::Deserialize;
use serde_json::from_slice;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use geojson::GeoJson;
use rand::{rng, Rng};

/// Geographic hierarchy levels: world -> continent -> country
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeoLevel {
    World,
    Continent,
    Country,
}

/// Metadata for a country loaded from `country_info.json`
#[derive(Clone, Debug, Deserialize)]
pub struct CountryInfo {
    pub name: String,
    pub capital: String,
    pub area: f64,
    pub population: u64,
    pub currency: String,
}

/// Caches loaded data: directory base, index of lists, optional country info, and fun facts
pub struct DataCache {
    base: PathBuf,
    index: BTreeMap<(GeoLevel, String), Vec<String>>,
    country_info: Option<BTreeMap<String, CountryInfo>>,
    funfacts: BTreeMap<String, Vec<String>>,
}

impl DataCache {
    /// Create a new DataCache, ensuring base directory and loading JSON files if present
    pub fn new<P: AsRef<Path>>(base: P) -> Result<Self, Box<dyn std::error::Error>> {
        let base = base.as_ref().to_path_buf();
        fs::create_dir_all(&base)?;

        // Attempt to load country metadata
        let country_info = fs::read(base.join("country_info.json"))
            .ok()
            .and_then(|b| from_slice::<BTreeMap<String, CountryInfo>>(&b).ok());

        // Load fun facts or default to empty map
        let funfacts = fs::read(base.join("funfacts.json"))
            .ok()
            .and_then(|b| from_slice::<BTreeMap<String, Vec<String>>>(&b).ok())
            .unwrap_or_default();

        Ok(Self { base, index: BTreeMap::new(), country_info, funfacts })
    }

    /// Load a JSON list for the given level and key, caching the result
    pub fn load_list(&mut self, level: GeoLevel, key: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let skey = key.to_lowercase().replace(' ', "_").replace(['(', ')'], "");
        let prefix = match level {
            GeoLevel::World => "continent",
            GeoLevel::Continent | GeoLevel::Country => "country",
        };
        let filename = format!("{}_{}.json", prefix, skey);
        let data = fs::read(self.base.join(&filename))?;
        let list: Vec<String> = from_slice(&data)?;
        self.index.insert((level, key.to_string()), list.clone());
        Ok(list)
    }

    /// Load GeoJSON data for the specified level and key
    pub fn load_geojson(&self, level: &GeoLevel, key: &str) -> Result<GeoJson, Box<dyn std::error::Error>> {
        let skey = key.to_lowercase().replace(' ', "_").replace(['(', ')'], "");
        let prefix = match level {
            GeoLevel::World => "continent",
            GeoLevel::Continent | GeoLevel::Country => "country",
        };
        let filename = format!("{}_{}.geojson", prefix, skey);
        let txt = fs::read_to_string(self.base.join(&filename))?;
        Ok(GeoJson::from_str(&txt)?)
    }

    /// Retrieve country metadata by key, if loaded
    pub fn load_country_info(&self, key: &str) -> Option<&CountryInfo> {
        let skey = key.to_lowercase().replace(' ', "_").replace(['(', ')'], "");
        self.country_info.as_ref()?.get(&skey)
    }

    /// Return a random fun fact for the given key, if available (1 of 3)
    pub fn random_funfact(&self, key: &str) -> Option<String> {
        let skey = key.to_lowercase().replace(' ', "_");
        self.funfacts.get(&skey).and_then(|facts| {
            if facts.is_empty() {
                None
            } else {
                let mut rng = rng();
                let idx = rng.random_range(0..facts.len());
                Some(facts[idx].clone())
            }
        })
    }

    /// Build a mapping of continents to their countries
    pub fn load_continent_mappings(&mut self) -> Result<HashMap<String, HashSet<String>>, Box<dyn std::error::Error>> {
        let mut result = HashMap::new();
        let continents = self.load_list(GeoLevel::World, "world")?;
        for continent in continents {
            if let Ok(countries) = self.load_list(GeoLevel::Continent, &continent) {
                result.insert(continent, countries.into_iter().collect());
            }
        }
        Ok(result)
    }
}
