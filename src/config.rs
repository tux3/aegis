use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Deserialize)]
pub struct Config {}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Config {
        let contents = std::fs::read_to_string(path.as_ref())
            .unwrap_or_else(|_| panic!("Failed to read config file: {}", path.as_ref().display()));
        toml::from_str(&contents).expect("Invalid config file format")
    }
}

pub fn default_path() -> PathBuf {
    let dirs = match directories_next::ProjectDirs::from("", "tux3", "aegiscli") {
        Some(dirs) => dirs,
        None => panic!("Couldn't locate config directory"),
    };
    dirs.config_dir().join("conf.toml")
}
