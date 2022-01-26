use crate::Config;
use aegislib::client::{register_device, ClientError, DeviceClient, StatusCode};
use aegislib::command::server::ServerCommand;
use aegislib::crypto::Keypair;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

/// If we get 403 Forbidden when connecting to the server, the device hasn't been approved by an admin
/// This is the cooldown before we periodically retry connecting to the server websocket
const FORBIDDEN_CONNECT_COOLDOWN: Duration = Duration::from_secs(15);

async fn register(config: &Config, key: &Keypair) -> Result<()> {
    // register_device considers CONFLICT as an error, but for our purpose it means the device
    // is already known by the server, we just need admin approval before we can connect
    // If register_device returns anything else, we actually failed to register and should exit
    match register_device(&config.into(), &config.device_name, &key.public).await {
        Ok(_) => Ok(()),
        Err(ClientError::Http(e)) if e.code == StatusCode::CONFLICT => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub async fn connect(
    config: &Config,
    mut key: Keypair,
    event_tx: Sender<ServerCommand>,
) -> Result<DeviceClient> {
    let mut has_registered = false;
    loop {
        match DeviceClient::new(&config.into(), key, Some(event_tx.clone())).await {
            Ok(c) => return Ok(c),
            Err((_, ClientError::Other(err))) => return Err(err),
            Err((err_key, ClientError::Http(err))) => {
                if err.code == StatusCode::FORBIDDEN {
                    if !has_registered {
                        tracing::warn!("Device rejected by server, please authorize the device with the admin interface");
                        register(config, &err_key).await?;
                        has_registered = true;
                    }
                    sleep(FORBIDDEN_CONNECT_COOLDOWN).await;
                } else {
                    return Err(err.into());
                }
                key = err_key;
            }
        }
    }
}
