use aegislib::crypto::{sign_keypair_from_file, SigningKey};
use anyhow::{Context, Result};
use std::path::Path;

pub fn get_or_create_keys(path: &Path) -> Result<SigningKey> {
    Ok(if path.exists() {
        sign_keypair_from_file(path)?
    } else {
        let dir = path.canonicalize().unwrap_or_else(|_| path.to_owned());
        let dir = dir
            .parent()
            .context("Failed to get parent dir of device key path")?;
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create device key directory {}", dir.display()))?;

        let sign_kp = aegislib::crypto::random_sign_keypair();
        std::fs::write(path, sign_kp.to_bytes())
            .with_context(|| format!("Failed to write device key file {}", path.display()))?;
        sign_kp
    })
}
