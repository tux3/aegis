use serde::Deserialize;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Context, Result};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub use_tls: bool,
    pub server_addr: String,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Config> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow!("Failed to read config file {}: {}", path.as_ref().display(), e))?;
        toml::from_str(&contents).context("Invalid config file format")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            use_tls: true,
            server_addr: "https://alacrem.net/aegis".to_string()
        }
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
