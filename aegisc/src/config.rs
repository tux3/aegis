use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub device_name: String,
    pub use_tls: bool,
    pub server_addr: String,
    pub device_key_path: PathBuf,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Config> {
        let contents = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            anyhow!(
                "Failed to read config file {}: {}",
                path.as_ref().display(),
                e
            )
        })?;
        toml::from_str(&contents).context("Invalid config file format")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_name: std::fs::read_to_string("/etc/hostname")
                .expect("Failed to read hostname")
                .trim()
                .to_owned(),
            use_tls: true,
            server_addr: "alacrem.net/aegis".to_string(),
            device_key_path: "/var/lib/aegisc/device.key".into(),
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
