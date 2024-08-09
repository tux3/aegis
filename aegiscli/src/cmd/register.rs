use crate::config::Config;
use aegislib::client::register_device;
use aegislib::crypto::sign_keypair_from_file;
use anyhow::Result;
use clap::ArgMatches;
use std::path::PathBuf;

pub async fn register(config: &Config, args: &ArgMatches) -> Result<()> {
    let name: &String = args.get_one("name").unwrap();
    let kp = sign_keypair_from_file(args.get_one::<PathBuf>("key").unwrap())?;
    register_device(&config.into(), name, &kp.verifying_key()).await?;
    Ok(())
}
