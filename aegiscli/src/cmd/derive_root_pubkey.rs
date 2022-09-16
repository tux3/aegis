use crate::config::Config;
use aegislib::crypto::RootKeys;
use anyhow::Result;
use clap::ArgMatches;

pub async fn derive_root_pubkey(_config: &Config, args: &ArgMatches) -> Result<()> {
    let password = args
        .value_of("password")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            dialoguer::Password::new()
                .with_prompt("Password")
                .interact()
                .unwrap()
        });
    let keys = RootKeys::derive(&password);
    let pubkey = base64::encode_config(keys.sig.public, base64::URL_SAFE_NO_PAD);
    println!("Root public signature key: {pubkey}");
    Ok(())
}
