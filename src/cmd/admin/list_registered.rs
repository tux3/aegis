use crate::config::Config;
use aegislib::client::AdminClient;
use anyhow::Result;
use clap::ArgMatches;
use cli_table::{print_stdout, Cell, Style, Table};
use sodiumoxide::base64;

pub async fn list_registered(
    _config: &Config,
    mut client: AdminClient,
    _args: &ArgMatches,
) -> Result<()> {
    let devices = client.list_registered().await?;
    let table = devices
        .into_iter()
        .map(|dev| {
            vec![
                base64::encode(dev.pubkey, base64::Variant::UrlSafeNoPadding),
                dev.name,
                format!("{}", dev.created_at),
            ]
        })
        .table()
        .title(vec![
            "Pubkey".cell().bold(true),
            "Name".cell().bold(true),
            "Created at".cell().bold(true),
        ]);
    print_stdout(table)?;
    Ok(())
}
