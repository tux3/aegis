use crate::config::Config;
use anyhow::Result;
use clap::ArgMatches;
use std::path::PathBuf;

pub async fn gen_device_key(_config: &Config, args: &ArgMatches) -> Result<()> {
    let output: &PathBuf = args.get_one("output").unwrap();
    let sign_kp = aegislib::crypto::random_sign_keypair();
    std::fs::write(output, sign_kp.to_bytes())?;
    Ok(())
}
