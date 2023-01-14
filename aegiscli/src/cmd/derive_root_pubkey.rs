use crate::config::Config;
use aegislib::crypto::RootKeys;
use anyhow::Result;
use base64::prelude::*;
use clap::ArgMatches;

pub async fn derive_root_pubkey(_config: &Config, args: &ArgMatches) -> Result<()> {
    let password = args
        .get_one::<String>("password")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            dialoguer::Password::new()
                .with_prompt("Password")
                .interact()
                .unwrap()
        });
    let keys = RootKeys::derive(&password);
    let pubkey = BASE64_URL_SAFE_NO_PAD.encode(keys.sig.public);
    println!("Root public signature key: {pubkey}");
    Ok(())
}
