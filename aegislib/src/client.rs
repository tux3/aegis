mod api_client;

pub use api_client::*;
use base64::prelude::*;
pub use reqwest::StatusCode;
use thiserror::Error;

mod dev_client;
pub use dev_client::*;

mod admin_client;
pub use admin_client::*;

mod rest_client;
pub use rest_client::*;

mod ws_client;
pub use ws_client::*;

#[derive(Debug, Clone, Error)]
#[error("Client HTTP error {}: {}", u16::from(*.code), .message.as_deref().unwrap_or_else(|| .code.canonical_reason().unwrap_or("<unknown status code>"))
)]
pub struct ClientHttpError {
    pub code: StatusCode,
    message: Option<String>,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Http(#[from] ClientHttpError),
    #[error("websocket disconnected: {0}")]
    WebsocketDisconnected(anyhow::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_addr: String,
    pub use_tls: bool,
    pub use_rest: bool,
}

pub async fn register_device(
    config: &ClientConfig,
    name: &str,
    pk: &ed25519_dalek::VerifyingKey,
) -> Result<(), ClientError> {
    let pk = BASE64_URL_SAFE_NO_PAD.encode(pk);
    let client = reqwest::Client::new();
    let proto = if config.use_tls {
        "https://"
    } else {
        "http://"
    };
    let reply = client
        .post(format!(
            "{proto}{addr}/register/{pk}/name/{name}",
            addr = &config.server_addr
        ))
        .send()
        .await
        .map_err(anyhow::Error::from)?;
    if reply.status().as_u16() == StatusCode::CONFLICT {
        return Err(ClientError::Http(ClientHttpError {
            code: StatusCode::CONFLICT,
            message: Some("Device already exists".into()),
        }));
    }
    if !reply.status().is_success() {
        return Err(ClientError::Http(ClientHttpError {
            code: reply.status(),
            message: reply.text().await.ok(),
        }));
    }
    Ok(())
}
