use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub struct GDPData {
    // Country code -> (year -> value)
    data: HashMap<String, HashMap<String, f64>>,
    // Country name -> country code (both original and lowercase for better matching)
    country_codes: HashMap<String, String>,
    // For debugging
    country_names: Vec<String>,
}

impl GDPData {
    pub fn new<P: AsRef<Path>>(csv_path: P) -> io::Result<Self> {
        let file = File::open(csv_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        
        // Skip header lines
        for _ in 0..5 {
            let _ = lines.next();
        }
        
        let mut data = HashMap::new();
        let mut country_codes = HashMap::new();
        let mut country_names = Vec::new();
        
        // Process data lines
        for line_result in lines {
            if let Ok(line) = line_result {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() < 5 {
                    continue;
                }
                
                // Extract country info
                let country_name = parts[0].trim_matches('"');
                let country_code = parts[1].trim_matches('"');
                
                country_codes.insert(country_name.to_string(), country_code.to_string());
                // Also insert lowercase version for case-insensitive matching
                country_codes.insert(country_name.to_lowercase(), country_code.to_string());
                country_names.push(country_name.to_string());
                
                // Create year->value map for this country
                let mut year_values = HashMap::new();
                
                // Start from index 4 which is year 1960
                for (i, value) in parts.iter().enumerate().skip(4) {
                    if i >= 68 { // Don't go beyond 2024
                        break;
                    }
                    
                    let year = (1960 + (i - 4)).to_string();
                    let value = value.trim_matches('"');
                    
                    if !value.is_empty() {
                        if let Ok(gdp) = value.parse::<f64>() {
                            year_values.insert(year, gdp);
                        }
                    }
                }
                
                data.insert(country_code.to_string(), year_values);
            }
        }
        
        eprintln!("Loaded GDP data for {} countries", country_names.len());
        
        Ok(Self { data, country_codes, country_names })
    }
    
    pub fn get_latest_gdp(&self, country_name: &str) -> Option<(String, f64)> {
        // Try exact match first
        let mut code = self.country_codes.get(country_name);
        
        // If that fails, try lowercase match
        if code.is_none() {
            code = self.country_codes.get(&country_name.to_lowercase());
        }
        
        // If still no match, try fuzzy matching
        if code.is_none() {
            for available_name in &self.country_names {
                if available_name.contains(country_name) || country_name.contains(available_name) {
                    code = self.country_codes.get(available_name);
                    if code.is_some() {
                        break;
                    }
                }
            }
        }
        
        // Get GDP data for this country if we found a code
        let code = code?;
        let gdp_data = self.data.get(code)?;
        
        // Find latest year with data
        let mut latest_year = None;
        let mut latest_value = 0.0;
        
        for (year, value) in gdp_data {
            if latest_year.is_none() || year > latest_year.as_ref().unwrap() {
                latest_year = Some(year.clone());
                latest_value = *value;
            }
        }
        
        latest_year.map(|year| (year, latest_value))
    }
    
    pub fn get_all_gdp_data(&self, country_name: &str) -> Option<HashMap<String, f64>> {
        // Try exact match first
        let mut code = self.country_codes.get(country_name);
        
        // If that fails, try lowercase match
        if code.is_none() {
            code = self.country_codes.get(&country_name.to_lowercase());
        }
        
        // If still no match, try fuzzy matching
        if code.is_none() {
            for available_name in &self.country_names {
                if available_name.contains(country_name) || country_name.contains(available_name) {
                    code = self.country_codes.get(available_name);
                    if code.is_some() {
                        break;
                    }
                }
            }
        }
        
        // Get GDP data for this country if we found a code
        let code = code?;
        let gdp_data = self.data.get(code)?;
        
        Some(gdp_data.clone())
    }
    
    pub fn format_gdp_value(value: f64) -> String {
        if value >= 1_000_000_000_000.0 {
            format!("{:.2} bln USD", value / 1_000_000_000_000.0)
        } else if value >= 1_000_000_000.0 {
            format!("{:.2} mld USD", value / 1_000_000_000.0)
        } else if value >= 1_000_000.0 {
            format!("{:.2} mln USD", value / 1_000_000.0)
        } else {
            format!("{:.2} USD", value)
        }
    }
}