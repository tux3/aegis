use crate::client::{ApiClient, ClientConfig, ClientError, RestClient, WsClient};
use crate::command::device::{StatusArg, StatusReply};
use crate::crypto::randomized_signature;
use anyhow::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct DeviceClient {
    client: Box<dyn ApiClient>,
    api_base: String,
    key: ed25519_dalek::Keypair,
}

impl DeviceClient {
    pub async fn new(
        config: &ClientConfig,
        key: ed25519_dalek::Keypair,
    ) -> Result<Self, (ed25519_dalek::Keypair, ClientError)> {
        let api_base = if config.use_rest {
            let dev_pk = base64::encode_config(&key.public, base64::URL_SAFE_NO_PAD);
            format!("/device/{}/", dev_pk)
        } else {
            String::new()
        };
        let client: Box<dyn ApiClient> = if config.use_rest {
            Box::new(RestClient::new_client(config).await)
        } else {
            match WsClient::new_device_client(config, &key).await {
                Err(e) => return Err((key, e)),
                Ok(c) => Box::new(c),
            }
        };
        Ok(DeviceClient {
            client,
            api_base,
            key,
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
        let reply = self.client.request(&route, &signature, payload).await?;
        Ok(bincode::deserialize(&reply).map_err(Error::from)?)
    }

    pub async fn status(&mut self) -> Result<StatusReply, ClientError> {
        self.do_request("status", StatusArg {}).await
    }
}
