use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

/// Holds GDP values by country code and provides lookup by country name.
pub struct GDPData {
    /// Map from ISO country code to a sorted map of year -> GDP value.
    data: HashMap<String, BTreeMap<u16, f64>>,
    /// Map from country name (original and lowercase) to ISO country code.
    country_codes: HashMap<String, String>,
    /// List of original country names for simple fuzzy matching.
    country_names: Vec<String>,
}

impl GDPData {
    /// Load GDP CSV, skipping 5 header lines, and build in-memory data structures.
    pub fn new<P: AsRef<Path>>(csv_path: P) -> io::Result<Self> {
        let file = File::open(csv_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Skip metadata headers
        for _ in 0..5 { let _ = lines.next(); }

        let mut data = HashMap::new();
        let mut country_codes = HashMap::new();
        let mut country_names = Vec::new();

        // Parse each line as country, code, and yearly GDP values
        for line in lines.flatten() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 5 { continue; }

            let name = parts[0].trim_matches('"');
            let code = parts[1].trim_matches('"');

            // Register exact and lowercase name lookups
            country_codes.insert(name.to_string(), code.to_string());
            country_codes.insert(name.to_lowercase(), code.to_string());
            country_names.push(name.to_string());

            let mut by_year = BTreeMap::new();
            // Years start at 1960 from the fifth column
            for (i, raw) in parts.iter().enumerate().skip(4) {
                let year = 1960 + (i - 4);
                if year > 2024 { break; }
                let s = raw.trim_matches('"');
                if !s.is_empty() {
                    if let Ok(val) = s.parse::<f64>() {
                        by_year.insert(year as u16, val);
                    }
                }
            }

            data.insert(code.to_string(), by_year);
        }

        Ok(Self { data, country_codes, country_names })
    }

    /// Resolve a country name to its ISO code via exact, lowercase, or substring match.
    fn find_country_code(&self, query: &str) -> Option<&String> {
        // Try exact match
        if let Some(code) = self.country_codes.get(query) {
            return Some(code);
        }
        // Try lowercase match
        let lc = query.to_lowercase();
        if let Some(code) = self.country_codes.get(&lc) {
            return Some(code);
        }
        // Fallback to simple substring fuzzy match
        for name in &self.country_names {
            if name.contains(query) || query.contains(name) {
                if let Some(code) = self.country_codes.get(name) {
                    return Some(code);
                }
            }
        }
        None
    }

    /// Get the most recent year and GDP value for a given country name.
    pub fn get_latest_gdp(&self, country_name: &str) -> Option<(u16, f64)> {
        let code = self.find_country_code(country_name)?;
        let years = self.data.get(code)?;
        years.iter().next_back().map(|(&y, &v)| (y, v))
    }

    /// Access the full year -> GDP map for charting purposes.
    pub fn get_all_gdp_data(&self, country_name: &str) -> Option<&BTreeMap<u16, f64>> {
        let code = self.find_country_code(country_name)?;
        self.data.get(code)
    }

    /// Format a GDP value into a human-friendly string with units.
    pub fn format_gdp_value(val: f64) -> String {
        if val >= 1e12 {
            format!("{:.2} bln USD", val / 1e12)
        } else if val >= 1e9 {
            format!("{:.2} mld USD", val / 1e9)
        } else if val >= 1e6 {
            format!("{:.2} mln USD", val / 1e6)
        } else {
            format!("{:.2} USD", val)
        }
    }
}
