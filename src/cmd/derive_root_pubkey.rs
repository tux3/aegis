use crate::config::Config;
use aegislib::crypto::derive_root_keys;
use anyhow::Result;
use clap::ArgMatches;
use sodiumoxide::base64;

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
    let keys = derive_root_keys(&password)?;
    let sig = keys.sig.public_key();
    let pubkey = base64::encode(sig.as_ref(), base64::Variant::UrlSafeNoPadding);
    println!("Root public signature key: {}", pubkey);
    Ok(())
}
