use crate::config::Config;
use crate::dev_client::DeviceClient;
use anyhow::Result;
use clap::ArgMatches;

pub async fn status(_config: &Config, mut client: DeviceClient, _args: &ArgMatches) -> Result<()> {
    let status = client.status().await?;
    println!("Device status: {:#?}", status);
    Ok(())
}
