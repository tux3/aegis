use crate::client::{ApiClient, ClientConfig, ClientError, RestClient, WsClient};
use crate::command::device::{
    DeviceEvent, StatusArg, StatusReply, StoreCameraPictureArg, StoreCameraPictureReply,
};
use crate::command::server::ServerCommand;
use crate::crypto::randomized_signature;
use anyhow::{anyhow, Error};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tracing::error;

pub struct DeviceClient {
    client: Box<dyn ApiClient>,
    api_base: String,
    config: ClientConfig,
    key: ed25519_dalek::Keypair,
    event_tx: Option<Sender<ServerCommand>>,
}

impl DeviceClient {
    pub async fn new(
        config: &ClientConfig,
        key: ed25519_dalek::Keypair,
        event_tx: Option<Sender<ServerCommand>>,
    ) -> Result<Self, (ed25519_dalek::Keypair, ClientError)> {
        let api_base = if config.use_rest {
            let dev_pk = base64::encode_config(&key.public, base64::URL_SAFE_NO_PAD);
            format!("/device/{}/", dev_pk)
        } else {
            String::new()
        };
        let client = match Self::build_client(config, &key, event_tx.clone()).await {
            Ok(c) => c,
            Err(e) => return Err((key, e)),
        };
        Ok(DeviceClient {
            client,
            api_base,
            config: config.to_owned(),
            key,
            event_tx,
        })
    }

    async fn build_client(
        config: &ClientConfig,
        key: &ed25519_dalek::Keypair,
        event_tx: Option<Sender<ServerCommand>>,
    ) -> Result<Box<dyn ApiClient>, ClientError> {
        Ok(if config.use_rest {
            if event_tx.is_some() {
                return Err(anyhow!("Cannot receive events if config.use_rest is true").into());
            }
            Box::new(RestClient::new_client(config).await)
        } else {
            match WsClient::new_device_client(config, key, event_tx).await {
                Err(e) => return Err(e),
                Ok(c) => Box::new(c),
            }
        })
    }

    async fn do_request<R: DeserializeOwned>(
        &mut self,
        route: &str,
        arg: impl Serialize,
    ) -> Result<R, ClientError> {
        let route = format!("{}{}", &self.api_base, route);
        let payload = bincode::serialize(&arg).map_err(Error::from)?;
        let signature = randomized_signature(&self.key, route.as_bytes(), &payload);
        let reply = match self
            .client
            .request(&route, &signature, payload.clone())
            .await
        {
            Err(ClientError::WebsocketDisconnected(e)) => {
                error!("do_request: Websocket disconnected ({}), retrying once", e);
                self.client =
                    Self::build_client(&self.config, &self.key, self.event_tx.clone()).await?;

                match self.client.request(&route, &signature, payload).await {
                    Err(ClientError::WebsocketDisconnected(e)) => {
                        self.client =
                            Self::build_client(&self.config, &self.key, self.event_tx.clone())
                                .await?;
                        return Err(ClientError::WebsocketDisconnected(anyhow!(
                            "Websocket keeps disconnecting: {}",
                            e
                        )));
                    }
                    Err(e) => return Err(e),
                    Ok(r) => r,
                }
            }
            Err(e) => return Err(e),
            Ok(r) => r,
        };
        Ok(bincode::deserialize(&reply)
            .map_err(|e| anyhow!("do_request: Failed to deserialize reply: {}", e))?)
    }

    pub async fn status(&mut self) -> Result<StatusReply, ClientError> {
        self.do_request("status", StatusArg {}).await
    }

    pub async fn store_camera_picture(
        &mut self,
        jpeg_data: Vec<u8>,
    ) -> Result<StoreCameraPictureReply, ClientError> {
        self.do_request("store_camera_picture", StoreCameraPictureArg { jpeg_data })
            .await
    }

    pub async fn log_event(&mut self, event: DeviceEvent) -> Result<(), ClientError> {
        self.do_request("log_event", event).await
    }
}
