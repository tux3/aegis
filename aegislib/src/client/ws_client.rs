use crate::client::{ApiClient, ClientConfig};
use anyhow::{bail, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
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
    ) -> Result<Self> {
        let pk = base64::encode_config(&key.public, base64::URL_SAFE_NO_PAD);
        let proto = if config.use_tls { "wss://" } else { "ws://" };
        let ws_url = format!("{}{}/ws/{}", proto, &config.server_addr, pk);
        let (ws_stream, _) = connect_async(ws_url).await?;
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
    ) -> Result<Bytes> {
        let signature = base64::encode_config(signature, base64::URL_SAFE_NO_PAD).into_bytes();
        let mut msg = signature.to_vec();
        msg.extend_from_slice(format!(" {} ", handler).as_bytes());
        msg.extend_from_slice(&payload);
        self.write.send(Message::Binary(msg)).await?;

        let reply = match self.read.next().await {
            None => bail!("Connection closed by websocket peer"),
            Some(reply) => Bytes::from(reply?.into_data()),
        };
        let mut elems = reply.splitn(3, |&c| c == b' ');
        let (reply_id, status, reply_payload) =
            match (elems.next(), elems.next(), elems.next(), elems.next()) {
                (Some(msg_id), Some(handler), Some(data), None) => (msg_id, handler, data),
                _ => bail!("Invalid websocket reply"),
            };
        if reply_id != signature {
            bail!("Invalid message id in reply");
        }
        let reply_payload = reply.slice_ref(reply_payload);
        match status {
            b"ok" => Ok(reply_payload),
            b"err" => bail!(
                "Error response: {}",
                String::from_utf8_lossy(&reply_payload)
            ),
            _ => bail!(
                "Invalid websocket response status: {}",
                String::from_utf8_lossy(status)
            ),
        }
    }
}