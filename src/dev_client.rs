use crate::config::Config;
use crate::ws_client::WsClient;
use aegislib::command::device::{StatusArg, StatusReply};
use anyhow::Result;
use sodiumoxide::crypto::sign;

pub struct DeviceClient {
    ws_client: WsClient,
}

impl DeviceClient {
    pub async fn new(config: &Config, key: sign::SecretKey) -> Result<Self> {
        let ws_client = WsClient::new_device_client(config, key).await?;
        Ok(DeviceClient { ws_client })
    }

    pub async fn status(&mut self) -> Result<StatusReply> {
        let args = bincode::serialize(&StatusArg {})?;
        let reply = self.ws_client.request("status", &args).await?;
        Ok(bincode::deserialize(&reply)?)
    }
}
