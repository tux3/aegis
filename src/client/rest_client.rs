use crate::client::{ApiClient, ClientConfig};
use anyhow::{bail, Result};
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;
use sodiumoxide::base64;

pub struct RestClient {
    base_url: String,
    client: Client,
}

impl RestClient {
    pub async fn new_client(config: &ClientConfig) -> Result<Self> {
        let proto = if config.use_tls {
            "https://"
        } else {
            "http://"
        };
        let base_url = format!("{}{}", proto, &config.server_addr);
        let client = Client::new();

        Ok(Self { base_url, client })
    }
}

#[async_trait]
impl ApiClient for RestClient {
    async fn request(
        &mut self,
        handler: &str,
        signature: &[u8],
        payload: Vec<u8>,
    ) -> Result<Bytes> {
        let signature = base64::encode(signature, base64::Variant::UrlSafeNoPadding);

        let url = format!("{}{}", &self.base_url, handler);
        let reply = self
            .client
            .post(url)
            .bearer_auth(signature)
            .body(payload)
            .send()
            .await?;
        if !reply.status().is_success() {
            bail!("{}: {}", reply.status().as_str(), reply.text().await?);
        }
        Ok(reply.bytes().await?)
    }
}
