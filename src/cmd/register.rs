use crate::config::Config;
use aegislib::crypto::priv_sign_key_from_file;
use anyhow::bail;
use anyhow::Result;
use clap::ArgMatches;
use sodiumoxide::base64;

pub async fn register(config: &Config, args: &ArgMatches) -> Result<()> {
    let name = args.value_of("name").unwrap();
    let sk = priv_sign_key_from_file(args.value_of_os("key").unwrap())?;
    let pk = sk.public_key();
    let pk = base64::encode(pk.as_ref(), base64::Variant::UrlSafeNoPadding);
    let client = reqwest::Client::new();
    let proto = if config.use_tls {
        "https://"
    } else {
        "http://"
    };
    let reply = client
        .post(format!(
            "{}{}/register/{}/name/{}",
            proto, &config.server_addr, pk, name
        ))
        .send()
        .await?;
    if !reply.status().is_success() {
        bail!(
            "{}: {}",
            reply.status().as_str(),
            reply.text().await.unwrap_or_else(|_| "unknown".into())
        )
    }
    Ok(())
}
