use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub use_tls: bool,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Config {
        let contents = std::fs::read_to_string(path.as_ref())
            .unwrap_or_else(|_| panic!("Failed to read config file: {}", path.as_ref().display()));
        toml::from_str(&contents).expect("Invalid config file format")
    }
}

pub fn default_path() -> PathBuf {
    "/etc/aegisc.toml".into()
}

impl From<&Config> for aegislib::client::ClientConfig {
    fn from(config: &Config) -> Self {
        Self {
            server_addr: config.server_addr.clone(),
            use_tls: config.use_tls,
            use_rest: false,
        }
    }
}
