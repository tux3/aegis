use crate::client::{ApiClient, ClientConfig, ClientError, ClientHttpError};
use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::debug;

pub struct WsClient {
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WsClient {
    // Websockets are only compatible with device handlers
    pub async fn new_device_client(
        config: &ClientConfig,
        key: &ed25519_dalek::Keypair,
    ) -> Result<Self, ClientError> {
        let pk = base64::encode_config(&key.public, base64::URL_SAFE_NO_PAD);
        let proto = if config.use_tls { "wss://" } else { "ws://" };
        let ws_url = format!("{}{}/ws/{}", proto, &config.server_addr, pk);
        let ws_stream = match connect_async(ws_url).await {
            Err(WsError::Http(err)) => {
                return Err(ClientHttpError {
                    code: err.status(),
                    message: None,
                }
                .into())
            }
            Err(e) => return Err(ClientError::Other(Error::from(e))),
            Ok((ws_stream, _)) => ws_stream,
        };
        debug!("WebSocket handshake completed");

        let (write, read) = ws_stream.split();

        Ok(WsClient { write, read })
    }
}

#[async_trait]
impl ApiClient for WsClient {
    async fn request(
        &mut self,
        handler: &str,
        signature: &[u8],
        payload: Vec<u8>,
    ) -> Result<Bytes, ClientError> {
        let signature = base64::encode_config(signature, base64::URL_SAFE_NO_PAD).into_bytes();
        let mut msg = signature.to_vec();
        msg.extend_from_slice(format!(" {} ", handler).as_bytes());
        msg.extend_from_slice(&payload);
        self.write
            .send(Message::Binary(msg))
            .await
            .map_err(Error::from)?;

        let reply = match self.read.next().await {
            None => return Err(anyhow!("Connection closed by websocket peer").into()),
            Some(reply) => Bytes::from(reply.map_err(Error::from)?.into_data()),
        };
        let mut elems = reply.splitn(3, |&c| c == b' ');
        let (reply_id, status, reply_payload) =
            match (elems.next(), elems.next(), elems.next(), elems.next()) {
                (Some(msg_id), Some(handler), Some(data), None) => (msg_id, handler, data),
                _ => return Err(anyhow!("Invalid websocket reply").into()),
            };
        if reply_id != signature {
            return Err(anyhow!("Invalid message id in reply").into());
        }
        let reply_payload = reply.slice_ref(reply_payload);
        match status {
            b"ok" => Ok(reply_payload),
            b"err" => {
                return Err(anyhow!(
                    "Error response: {}",
                    String::from_utf8_lossy(&reply_payload)
                )
                .into())
            }
            _ => {
                return Err(anyhow!(
                    "Invalid websocket response status: {}",
                    String::from_utf8_lossy(status)
                )
                .into())
            }
        }
    }
}
