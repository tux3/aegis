use crate::config::Config;
use aegislib::client::AdminClient;
use aegislib::command::admin::SetStatusArg;
use anyhow::{bail, Result};
use clap::ArgMatches;

fn parse_bool(s: &str) -> Result<bool> {
    let s = s.to_lowercase();
    if s == "1" || s == "y" || s == "yes" || s == "t" || s == "true" {
        Ok(true)
    } else if s == "0" || s == "n" || s == "no" || s == "f" || s == "false" {
        Ok(false)
    } else {
        bail!("Invalid boolean value: {}", s);
    }
}

pub async fn set_status(
    _config: &Config,
    mut client: AdminClient,
    args: &ArgMatches,
) -> Result<()> {
    let name = args.value_of("name").unwrap();
    let vt_locked = args.value_of("vt-lock").map(parse_bool).transpose()?;
    let ssh_locked = args.value_of("ssh-lock").map(parse_bool).transpose()?;
    let status = client
        .set_status(SetStatusArg {
            dev_name: name.to_owned(),
            vt_locked,
            ssh_locked,
        })
        .await?;
    println!("New device status: {:#?}", status);
    Ok(())
}
