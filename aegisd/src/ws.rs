use crate::error::Error;
use crate::handler::device::{device_handler_iter, DeviceHandlerFn, DeviceId};
use aegislib::command::server::ServerCommand;
use aegislib::crypto::check_signature;
use anyhow::anyhow;
use async_stream::stream;
use axum::body::Bytes;
use axum::extract::ws::{close_code, CloseFrame, Message, WebSocket};
use dashmap::DashMap;
use ed25519_dalek::PublicKey;
use futures::pin_mut;
use futures::StreamExt;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::select;
use tokio::sync::mpsc::Sender;
use tracing::{error, info, warn};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const WS_TIMEOUT: Duration = Duration::from_secs(10);

lazy_static::lazy_static! {
    static ref HANDLER_MAP: HashMap<String, DeviceHandlerFn> = {
        let mut m = HashMap::new();
        for handler in device_handler_iter() {
            m.insert(handler.path.trim_start_matches('/').to_owned(), handler.handler);
        }
        m
    };

    static ref WS_CLIENT_MAP: DashMap<DeviceId, Sender<ServerCommand>> = DashMap::new();
}

pub fn ws_for_device(dev_id: DeviceId) -> Option<Sender<ServerCommand>> {
    WS_CLIENT_MAP.get(&dev_id).map(|a| a.clone())
}

async fn send_response(
    ws: &mut WebSocket,
    ok: bool,
    msg_id: &[u8],
    payload: &[u8],
) -> Result<(), Error> {
    let mut msg = msg_id.to_vec();
    if ok {
        msg.extend_from_slice(b" ok ");
    } else {
        msg.extend_from_slice(b" err ");
    };
    msg.extend_from_slice(payload);
    ws.send(Message::Binary(msg)).await?;
    Ok(())
}

async fn send_server_command(ws: &mut WebSocket, cmd: ServerCommand) -> Result<(), Error> {
    // WS server message format: <handler> <payload>
    let mut payload = b"server_command ".to_vec();
    bincode::serialize_into(&mut payload, &cmd).unwrap();
    ws.send(Message::Binary(payload)).await?;
    Ok(())
}

pub struct WsConn {
    db: PgPool,
    device_pk: PublicKey,
    device_id: DeviceId,
    last_heartbeat: Instant,
    remote_addr_untrusted: String,
}

impl WsConn {
    pub fn new(
        db: PgPool,
        device_pk: PublicKey,
        device_id: DeviceId,
        remote_addr_untrusted: String,
    ) -> WsConn {
        WsConn {
            db,
            device_pk,
            device_id,
            last_heartbeat: Instant::now(),
            remote_addr_untrusted,
        }
    }

    pub async fn handle(mut self, mut ws: WebSocket) -> Result<(), Error> {
        let (send_queue_tx, mut send_queue_rx) = tokio::sync::mpsc::channel(4);
        WS_CLIENT_MAP.insert(self.device_id, send_queue_tx);
        let heartbeat = stream! {
            loop {
                tokio::time::sleep(HEARTBEAT_INTERVAL).await;
                yield Message::Ping(b"ping".to_vec());
            }
        };
        pin_mut!(heartbeat);
        loop {
            select! {
                ping = heartbeat.next() => {
                    ws.send(ping.unwrap()).await?;
                },
                msg = send_queue_rx.recv() => {
                    let msg = msg.ok_or_else(|| anyhow!("Send queue tx dropped!"))?;
                    send_server_command(&mut ws, msg).await?;
                },
                msg = ws.recv() => {
                    let msg = match msg {
                        Some(Ok(msg)) => msg,
                        Some(Err(e)) => {
                            error!(remote_addr = &self.remote_addr_untrusted, "Protocol error: {e}");
                            break;
                        },
                        None => {
                            warn!(remote_addr = &self.remote_addr_untrusted, "Websocket connection closed");
                            break;
                        }
                    };
                    if let Err(close_msg) = self.handle_ws_msg(&mut ws, msg).await {
                        let _ = ws.send(Message::Close(close_msg)).await;
                        break;
                    }
                }
            }
            if Instant::now().duration_since(self.last_heartbeat) > WS_TIMEOUT {
                info!("{}: ping timeout", &self.remote_addr_untrusted);
                break;
            }
        }

        Ok(())
    }

    async fn handle_ws_msg(
        &mut self,
        ws: &mut WebSocket,
        msg: Message,
    ) -> Result<(), Option<CloseFrame<'static>>> {
        match msg {
            Message::Text(payload) => self.handle_message_data(ws, payload.into_bytes()).await,
            Message::Binary(payload) => self.handle_message_data(ws, payload).await,
            Message::Ping(msg) => {
                self.last_heartbeat = Instant::now();
                ws.send(Message::Pong(msg)).await.map_err(|e| {
                    Some(CloseFrame {
                        code: close_code::ABNORMAL,
                        reason: format!("Failed to respond to ping: {e}").into(),
                    })
                })?;
                Ok(())
            }
            Message::Pong(_) => {
                self.last_heartbeat = Instant::now();
                Ok(())
            }
            Message::Close(reason) => {
                let remote_addr = &self.remote_addr_untrusted;
                warn!(%remote_addr, "Closed websocket with reason: {reason:?}");
                Err(None)
            }
        }
    }

    async fn handle_message_data(
        &self,
        ws: &mut WebSocket,
        raw_payload: Vec<u8>,
    ) -> Result<(), Option<CloseFrame<'static>>> {
        // WS message format: <msg_id> <handler> <data>
        let raw_payload = Bytes::from(raw_payload);
        let mut elems = raw_payload.splitn(3, |&c| c == b' ');
        let remote_addr = self.remote_addr_untrusted.as_str();
        let (msg_id, handler, data) = match (elems.next(), elems.next(), elems.next(), elems.next())
        {
            (Some(msg_id), Some(handler), Some(data), None) => (msg_id, handler, data),
            _ => {
                warn!(
                    %remote_addr,
                    size = raw_payload.len(),
                    "Invalid websocket message"
                );
                return Err(Some(CloseFrame {
                    code: close_code::PROTOCOL,
                    reason: "Invalid websocket message".into(),
                }));
            }
        };
        let signature = match base64::decode_config(msg_id, base64::URL_SAFE_NO_PAD) {
            Ok(msg_id) => Bytes::from(msg_id),
            Err(_) => {
                warn!(%remote_addr, "Websocket msg_id is invalid base64");
                return Err(Some(CloseFrame {
                    code: close_code::PROTOCOL,
                    reason: "Websocket msg_id is invalid base64".into(),
                }));
            }
        };
        let handler = match std::str::from_utf8(handler) {
            Ok(handler) => handler,
            _ => {
                warn!(%remote_addr, "Websocket handler name is not valid UTF-8");
                return Err(Some(CloseFrame {
                    code: close_code::PROTOCOL,
                    reason: "Websocket handler name is not valid UTF-8".into(),
                }));
            }
        };

        // msg_id is actually also a randomized signature!
        if !check_signature(&self.device_pk, &signature, handler.as_bytes(), data) {
            warn!(%remote_addr, %handler, "Invalid websocket message signature");
            return Err(Some(CloseFrame {
                code: close_code::POLICY,
                reason: "invalid signature".into(),
            }));
        }

        let handler = match HANDLER_MAP.get(handler) {
            Some(handler) => handler,
            _ => {
                warn!(%remote_addr, "Websocket handler not found: {handler}");
                send_response(ws, false, msg_id, b"handler not found")
                    .await
                    .map_err(|_| None)?;
                return Ok(());
            }
        };

        let db = self.db.clone();
        let dev_id = self.device_id;
        let data = raw_payload.slice_ref(data);
        match handler(db, dev_id, data).await {
            Ok(reply) => send_response(ws, true, msg_id, &reply).await,
            Err(e) => send_response(ws, false, msg_id, format!("{e}").as_bytes()).await,
        }
        .map_err(|e| {
            Some(CloseFrame {
                code: close_code::ERROR,
                reason: format!("Failed to send handler response: {}", e).into(),
            })
        })?;
        Ok(())
    }
}
