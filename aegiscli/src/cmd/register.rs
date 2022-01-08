use crate::config::Config;
use aegislib::client::register_device;
use aegislib::crypto::sign_keypair_from_file;
use anyhow::Result;
use clap::ArgMatches;

pub async fn register(config: &Config, args: &ArgMatches) -> Result<()> {
    let name = args.value_of("name").unwrap();
    let kp = sign_keypair_from_file(args.value_of_os("key").unwrap())?;
    register_device(&config.into(), name, &kp.public).await?;
    Ok(())
}
