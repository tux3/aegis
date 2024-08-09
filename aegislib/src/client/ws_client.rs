use crate::client::ClientError::WebsocketDisconnected;
use crate::client::{ApiClient, ClientConfig, ClientError, ClientHttpError};
use crate::command::server::ServerCommand;
use anyhow::{anyhow, bail, Error, Result};
use async_trait::async_trait;
use base64::prelude::*;
use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, warn};

const PING_TIMEOUT: Duration = Duration::from_secs(10);

struct WsRequestReply {
    msg_id: Bytes,
    reply: Result<Bytes, ClientError>,
}

enum WsReceivedMessage {
    Ping,
    ServerCommand(ServerCommand),
    RequestReply(WsRequestReply),
}

pub struct WsClient {
    write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    request_tx: Sender<Vec<u8>>,
    response_rx: Receiver<Result<Bytes, ClientError>>,
}

impl WsClient {
    // Websockets are only compatible with device handlers
    pub async fn new_device_client(
        config: &ClientConfig,
        key: &ed25519_dalek::SigningKey,
        event_tx: Option<Sender<ServerCommand>>,
    ) -> Result<Self, ClientError> {
        let pk = BASE64_URL_SAFE_NO_PAD.encode(key.verifying_key());
        let proto = if config.use_tls { "wss://" } else { "ws://" };
        let ws_url = format!("{}{}/ws/{}", proto, &config.server_addr, pk);
        let ws_stream = Self::connect(&ws_url).await?;
        debug!("WsClient: WebSocket handshake completed");

        let (write, read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));
        let (request_tx, request_rx) = channel(1);
        let (response_tx, response_rx) = channel(1);

        {
            let write = write.clone();
            spawn(async move {
                Self::recv_messages(read, request_rx, response_tx, event_tx, ws_url, write).await
            });
        }
        Ok(WsClient {
            write,
            request_tx,
            response_rx,
        })
    }

    async fn connect(
        ws_url: &str,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, ClientError> {
        Ok(match connect_async(ws_url).await {
            Err(WsError::Http(err)) => {
                return Err(ClientHttpError {
                    code: err.status(),
                    message: None,
                }
                .into())
            }
            Err(e) => return Err(ClientError::Other(Error::from(e))),
            Ok((ws_stream, _)) => ws_stream,
        })
    }

    async fn connect_loop_forever(ws_url: &str) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
        loop {
            match Self::connect(ws_url).await {
                Ok(s) => break s,
                Err(e) => {
                    warn!("WsClient: Failed to connect to websocket: {e}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn recv_messages(
        mut read_stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        mut request_rx: Receiver<Vec<u8>>,
        response_tx: Sender<Result<Bytes, ClientError>>,
        event_tx: Option<Sender<ServerCommand>>,
        ws_connect_url: String,
        write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    ) {
        let mut last_ping_time = Instant::now();
        let mut last_request_id = None;
        let err = loop {
            let recv_fut = Self::recv_one_message(&mut read_stream);
            let recv_fut = timeout(PING_TIMEOUT, recv_fut);
            let maybe_msg = match recv_fut.await {
                Ok(m) => m,
                Err(_) => {
                    if Instant::now().duration_since(last_ping_time) >= PING_TIMEOUT {
                        warn!("WsClient: Websocket ping timeout");
                        let ws_stream = Self::connect_loop_forever(&ws_connect_url).await;
                        debug!("WsClient: WebSocket reconnected");
                        let (new_write, new_read) = ws_stream.split();
                        read_stream = new_read;
                        *write.lock().await = new_write;
                    }
                    continue;
                }
            };
            let msg = match maybe_msg {
                Ok(msg) => msg,
                Err(WebsocketDisconnected(e)) => {
                    error!("WsClient::recv_message: {e}");
                    let ws_stream = Self::connect_loop_forever(&ws_connect_url).await;
                    debug!("WsClient: WebSocket reconnected");
                    let (new_write, new_read) = ws_stream.split();
                    read_stream = new_read;
                    *write.lock().await = new_write;
                    continue;
                }
                Err(e) => break e,
            };
            if let Err(e) = Self::update_request_id(&mut last_request_id, &mut request_rx).await {
                break e;
            }
            match Self::parse_received_message(msg) {
                Ok(WsReceivedMessage::Ping) => {
                    last_ping_time = Instant::now();
                    let mut write = write.lock().await;
                    if let Err(e) = write
                        .send(Message::Pong(b"pong".to_vec()))
                        .await
                        .map_err(Error::from)
                    {
                        warn!("WsClient::recv_message: Failed to answer ping: {e}");
                    }
                }
                Ok(WsReceivedMessage::ServerCommand(cmd)) => {
                    if let Some(event_tx) = &event_tx {
                        if let Err(e) = event_tx.send(cmd).await {
                            error!("WsClient::recv_message: Failed to send server cmd: {e}");
                        }
                    }
                }
                Ok(WsReceivedMessage::RequestReply(reply)) => {
                    if last_request_id.as_deref() == Some(reply.msg_id.as_ref()) {
                        if let Err(e) = response_tx.send(reply.reply).await {
                            error!("WsClient::recv_message: Failed to send reply: {e}");
                            return;
                        }
                    } else {
                        warn!(
                            "WsClient::recv_message: Got reply for non-existent request {}",
                            std::str::from_utf8(&reply.msg_id).unwrap_or("<invalid UTF-8>")
                        )
                    }
                }
                Err(e) => warn!("WsClient::recv_message: Failed to parse received message: {e}"),
            }
        };

        error!("WsClient::recv_message: disconnected: {err}");
        if let Err(send_err) = response_tx
            .send(Err(WebsocketDisconnected(anyhow!(err))))
            .await
        {
            error!("WsClient::recv_message: Channel failed while trying to send error: {send_err}")
        }
    }

    async fn update_request_id(
        last_request_id: &mut Option<Vec<u8>>,
        recv: &mut Receiver<Vec<u8>>,
    ) -> Result<(), ClientError> {
        // If send fails, we can have a stale req id that will never receive a reply
        // Since we can only send one message at a time, only the last req id is still active
        loop {
            match recv.try_recv() {
                Ok(req_id) => *last_request_id = Some(req_id),
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err(anyhow!("WsClient::recv_message: request channel gone").into())
                }
            }
        }
    }

    async fn recv_one_message(
        read_stream: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Result<Bytes, ClientError> {
        let reply = match read_stream.next().await {
            None => {
                return Err(WebsocketDisconnected(anyhow!(
                    "Connection closed by websocket peer (end of stream)"
                )))
            }
            Some(Err(WsError::ConnectionClosed)) => {
                return Err(WebsocketDisconnected(anyhow!(
                    "Connection closed by websocket peer"
                )))
            }
            Some(Err(WsError::Io(e))) => return Err(WebsocketDisconnected(anyhow!(e))),
            Some(reply) => Bytes::from(reply.map_err(Error::from)?.into_data()),
        };
        Ok(reply)
    }

    fn parse_received_message(data: Bytes) -> Result<WsReceivedMessage> {
        // The format for server commands is: "server_command" <payload>
        // For request replies, it's: <msg_id> <"ok"|"err"> <payload>
        // msg_ids are base64 of ed25519 signature, so we know they can't conflict (different len)

        let first_field = data.split(|&c| c == b' ').next().unwrap();
        if first_field == b"server_command" {
            bincode::deserialize(&data[first_field.len() + 1..])
                .map(WsReceivedMessage::ServerCommand)
                .map_err(From::from)
        } else if first_field == b"ping" {
            Ok(WsReceivedMessage::Ping)
        } else {
            let msg_id = data.slice_ref(first_field);
            if data.len() < msg_id.len() + 1 {
                bail!("Invalid message (no room for second field)")
            }
            let remaining_fields = &data[msg_id.len() + 1..];
            let mut elems = remaining_fields.splitn(2, |&c| c == b' ');
            let (status, reply_payload) = match (elems.next(), elems.next(), elems.next()) {
                (Some(handler), Some(data), None) => (handler, data),
                _ => bail!("Invalid message (wrong number of fields)"),
            };
            let reply_payload = data.slice_ref(reply_payload);
            let reply = match status {
                b"ok" => Ok(reply_payload),
                b"err" => Err(anyhow!(
                    "Error response: {}",
                    String::from_utf8_lossy(&reply_payload)
                )
                .into()),
                _ => Err(anyhow!(
                    "Invalid websocket response status: {}",
                    String::from_utf8_lossy(status)
                )
                .into()),
            };
            Ok(WsReceivedMessage::RequestReply(WsRequestReply {
                msg_id,
                reply,
            }))
        }
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
        let signature = BASE64_URL_SAFE_NO_PAD.encode(signature).into_bytes();
        let mut msg = signature.to_vec();
        msg.extend_from_slice(format!(" {handler} ").as_bytes());
        msg.extend_from_slice(&payload);
        self.request_tx
            .send(signature)
            .await
            .map_err(|_| ClientError::WebsocketDisconnected(anyhow!("request channel dropped!")))?;
        {
            let mut write = self.write.lock().await;
            write
                .send(Message::Binary(msg))
                .await
                .map_err(Error::from)?;
        }

        match self.response_rx.recv().await {
            None => Err(ClientError::WebsocketDisconnected(anyhow!(
                "Receiver task is gone, cannot read reply"
            ))),
            Some(reply) => reply,
        }
    }
}
