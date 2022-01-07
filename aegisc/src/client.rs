use aegislib::client::DeviceClient;
use aegislib::crypto::Keypair;
use crate::Config;
use anyhow::Result;

pub async fn connect(config: &Config, dev_key: Keypair) -> Result<DeviceClient> {
    let client = DeviceClient::new(&config.into(), dev_key).await?;
    // TODO: Catch 403 errors and try to register
    Ok(client)
}
