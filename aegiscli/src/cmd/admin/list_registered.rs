use crate::config::Config;
use aegislib::client::AdminClient;
use anyhow::Result;
use base64::prelude::*;
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use cli_table::{print_stdout, Cell, Style, Table};

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
                BASE64_URL_SAFE_NO_PAD.encode(dev.pubkey),
                dev.name,
                format!("{:?}", DateTime::<Utc>::from(dev.created_at)),
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
