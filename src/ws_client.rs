use crate::config::Config;
use aegislib::crypto::randomized_signature;
use anyhow::{bail, Result};
use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use sodiumoxide::base64;
use sodiumoxide::crypto::sign;
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::debug;

pub struct WsClient {
    key: sign::SecretKey,
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WsClient {
    pub async fn new_device_client(config: &Config, key: sign::SecretKey) -> Result<Self> {
        let pk = base64::encode(key.public_key(), base64::Variant::UrlSafeNoPadding);
        let ws_url = format!("ws://{}/ws/{}", &config.server_addr, pk);
        let (ws_stream, _) = connect_async(ws_url).await?;
        debug!("WebSocket handshake completed");

        let (write, read) = ws_stream.split();

        Ok(WsClient { key, write, read })
    }

    pub async fn request(&mut self, handler: &str, payload: &[u8]) -> Result<Bytes> {
        let signature = randomized_signature(&self.key, payload);
        let signature = base64::encode(signature, base64::Variant::UrlSafeNoPadding).into_bytes();
        let mut msg = signature.to_vec();
        msg.extend_from_slice(format!(" {} ", handler).as_bytes());
        msg.extend_from_slice(payload);
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
