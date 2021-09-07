use serde::Deserialize;
use std::path::Path;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub db_host: String,
    pub db_name: String,
    pub db_user: String,
    pub db_pass: String,
    #[serde(default = "db_max_conn_default")]
    pub db_max_conn: u32,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Config {
        let contents = std::fs::read_to_string(path.as_ref()).expect("Failed to read config file");
        toml::from_str(&contents).expect("Invalid config file format")
    }
}

fn db_max_conn_default() -> u32 {
    16
}
