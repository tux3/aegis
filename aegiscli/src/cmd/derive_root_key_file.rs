use crate::config::Config;
use aegislib::crypto::RootKeys;
use anyhow::Result;
use clap::ArgMatches;
use std::path::PathBuf;

pub async fn derive_root_key_file(_config: &Config, args: &ArgMatches) -> Result<()> {
    let out_path: &PathBuf = args.get_one("output").unwrap();
    let password = args
        .get_one::<String>("password")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            dialoguer::Password::new()
                .with_prompt("Password")
                .interact()
                .unwrap()
        });
    let keys = bincode::serialize(&RootKeys::derive(&password))?;
    std::fs::write(out_path, keys)?;
    Ok(())
}
