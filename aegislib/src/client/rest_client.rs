use crate::client::{ApiClient, ClientConfig, ClientError, ClientHttpError};
use anyhow::{Error, Result};
use async_trait::async_trait;
use base64::prelude::*;
use bytes::Bytes;
use reqwest::Client;

pub struct RestClient {
    base_url: String,
    client: Client,
}

impl RestClient {
    pub async fn new_client(config: &ClientConfig) -> Self {
        let proto = if config.use_tls {
            "https://"
        } else {
            "http://"
        };
        let base_url = format!("{}{}", proto, &config.server_addr);
        let client = Client::new();

        Self { base_url, client }
    }
}

#[async_trait]
impl ApiClient for RestClient {
    async fn request(
        &mut self,
        handler: &str,
        signature: &[u8],
        payload: Vec<u8>,
    ) -> Result<Bytes, ClientError> {
        let signature = BASE64_URL_SAFE_NO_PAD.encode(signature);

        let url = format!("{}{}", &self.base_url, handler);
        let reply = self
            .client
            .post(url)
            .bearer_auth(signature)
            .body(payload)
            .send()
            .await
            .map_err(Error::from)?;
        if !reply.status().is_success() {
            return Err(ClientHttpError {
                code: reply.status(),
                message: reply.text().await.ok(),
            }
            .into());
        }
        Ok(reply.bytes().await.map_err(Error::from)?)
    }
}
