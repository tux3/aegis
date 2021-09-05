use serde::Deserialize;
use std::path::Path;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub port: u16,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Config {
        let contents = std::fs::read_to_string(path.as_ref()).expect("Failed to read config file");
        toml::from_str(&contents).expect("Invalid config file format")
    }
}
