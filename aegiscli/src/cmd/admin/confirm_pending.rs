use crate::config::Config;
use aegislib::client::AdminClient;
use anyhow::Result;
use clap::ArgMatches;

pub async fn confirm_pending(
    _config: &Config,
    mut client: AdminClient,
    args: &ArgMatches,
) -> Result<()> {
    let name = args.value_of("name").unwrap();
    client.confirm_pending(name.to_owned()).await?;
    Ok(())
}
