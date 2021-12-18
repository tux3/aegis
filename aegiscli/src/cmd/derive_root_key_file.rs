use crate::config::Config;
use aegislib::crypto::RootKeys;
use anyhow::Result;
use clap::ArgMatches;

pub async fn derive_root_key_file(_config: &Config, args: &ArgMatches) -> Result<()> {
    let out_path = args.value_of_os("output").unwrap();
    let password = args
        .value_of("password")
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
