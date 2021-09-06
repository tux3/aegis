use crate::config::Config;
use anyhow::Result;
use clap::ArgMatches;

pub async fn gen_device_key(_config: &Config, args: &ArgMatches) -> Result<()> {
    let output = args.value_of_os("output").unwrap();
    let secret_key = sodiumoxide::crypto::sign::gen_keypair().1 .0;
    std::fs::write(output, secret_key)?;
    Ok(())
}
