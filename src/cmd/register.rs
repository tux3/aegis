use crate::config::Config;
use aegislib::crypto::sign_keypair_from_file;
use anyhow::bail;
use anyhow::Result;
use clap::ArgMatches;

pub async fn register(config: &Config, args: &ArgMatches) -> Result<()> {
    let name = args.value_of("name").unwrap();
    let kp = sign_keypair_from_file(args.value_of_os("key").unwrap())?;
    let pk = base64::encode_config(&kp.public, base64::URL_SAFE_NO_PAD);
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
