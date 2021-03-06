use crate::handler::device::{device_handler_iter, DeviceHandlerFn, DeviceId};
use actix::dev::Stream;
use actix::{
    Actor, ActorContext, Addr, AsyncContext, ContextFutureSpawner, Handler, Message, StreamHandler,
    WrapFuture,
};
use actix_http::error::PayloadError;
use actix_http::ws;
use actix_http::ws::Codec;
use actix_web::web::{Bytes, BytesMut};
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web_actors::ws::{handshake, WebsocketContext};
use aegislib::command::server::ServerCommand;
use aegislib::crypto::check_signature;
use dashmap::DashMap;
use ed25519_dalek::PublicKey;
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const WS_TIMEOUT: Duration = Duration::from_secs(10);
const WS_PAYLOAD_MAX_SIZE: usize = 2 * 1024 * 1024;

lazy_static::lazy_static! {
    static ref HANDLER_MAP: HashMap<String, DeviceHandlerFn> = {
        let mut m = HashMap::new();
        for handler in device_handler_iter() {
            m.insert(handler.path.trim_start_matches('/').to_owned(), handler.handler);
        }
        m
    };

    static ref WS_CLIENT_MAP: DashMap<DeviceId, Addr<WsConn>> = DashMap::new();
}

pub fn ws_for_device(dev_id: DeviceId) -> Option<Addr<WsConn>> {
    WS_CLIENT_MAP.get(&dev_id).map(|a| a.clone())
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsResponse {
    is_ok: bool,
    msg_id: Bytes,
    payload: Bytes,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsServerCommand {
    payload: Bytes,
}

impl<T: Into<ServerCommand> + Serialize> From<T> for WsServerCommand {
    fn from(cmd: T) -> Self {
        let cmd = cmd.into();
        let payload = Bytes::from(bincode::serialize(&cmd).unwrap());
        Self { payload }
    }
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

    pub fn start<S>(self, req: &HttpRequest, stream: S) -> Result<HttpResponse, Error>
    where
        S: Stream<Item = Result<Bytes, PayloadError>> + 'static,
    {
        let mut res = handshake(req)?;
        let codec = Codec::new().max_size(WS_PAYLOAD_MAX_SIZE);
        Ok(res.streaming(WebsocketContext::with_codec(self, stream, codec)))
    }

    fn start_heartbeat(&self, ctx: &mut WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > WS_TIMEOUT {
                info!("{}: ping timeout", &act.remote_addr_untrusted);
                ctx.stop();
                return;
            }

            ctx.ping(b"ping");
        });
    }

    fn handle_message_data(&self, ctx: &mut WebsocketContext<Self>, raw_payload: Bytes) {
        // WS message format: <msg_id> <handler> <data>
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
                ctx.close(Some(ws::CloseCode::Protocol.into()));
                return;
            }
        };
        let msg_id = raw_payload.slice_ref(msg_id);
        let signature = match base64::decode_config(&msg_id, base64::URL_SAFE_NO_PAD) {
            Ok(msg_id) => Bytes::from(msg_id),
            Err(_) => {
                warn!(%remote_addr, "Websocket msg_id is invalid base64");
                ctx.close(Some(ws::CloseCode::Protocol.into()));
                return;
            }
        };
        let data = raw_payload.slice_ref(data);
        let handler = match std::str::from_utf8(handler) {
            Ok(handler) => handler,
            _ => {
                warn!(%remote_addr, "Websocket handler name is not valid UTF-8");
                ctx.close(Some(ws::CloseCode::Protocol.into()));
                return;
            }
        };

        // msg_id is actually also a randomized signature!
        if !check_signature(&self.device_pk, &signature, handler.as_bytes(), &data) {
            warn!(%remote_addr, %handler, "Invalid websocket message signature");
            ctx.notify(WsResponse {
                is_ok: false,
                msg_id,
                payload: "invalid signature".into(),
            });
            return;
        }

        let handler = match HANDLER_MAP.get(handler) {
            Some(handler) => handler,
            _ => {
                warn!(%remote_addr, "Websocket handler not found: {handler}");
                ctx.notify(WsResponse {
                    is_ok: false,
                    msg_id,
                    payload: "handler not found".into(),
                });
                return;
            }
        };

        let self_addr = ctx.address().recipient();
        let db = self.db.clone();
        let dev_id = self.device_id;
        let fut = async move {
            let reply_bytes = match handler(db, dev_id, data).await {
                Ok(reply) => WsResponse {
                    is_ok: true,
                    msg_id,
                    payload: reply,
                },
                Err(e) => WsResponse {
                    is_ok: false,
                    msg_id,
                    payload: format!("{e}").into(),
                },
            };
            self_addr.send(reply_bytes).await.unwrap_or_else(|e| {
                warn!("Failed to send websocket reply to actor: {e}");
            })
        };
        fut.into_actor(self).spawn(ctx);
    }
}

impl Handler<WsResponse> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: WsResponse, ctx: &mut Self::Context) {
        let mut ws_header = BytesMut::from(msg.msg_id.as_ref());
        if msg.is_ok {
            ws_header.extend_from_slice(b" ok ");
        } else {
            ws_header.extend_from_slice(b" err ");
        };
        ctx.write_raw(ws::Message::Continuation(ws::Item::FirstBinary(
            ws_header.into(),
        )));
        ctx.write_raw(ws::Message::Continuation(ws::Item::Last(msg.payload)));
    }
}

impl Handler<WsServerCommand> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: WsServerCommand, ctx: &mut Self::Context) {
        // WS server message format: <handler> <payload>
        ctx.write_raw(ws::Message::Continuation(ws::Item::FirstBinary(
            "server_command ".into(),
        )));
        ctx.write_raw(ws::Message::Continuation(ws::Item::Last(msg.payload)));
    }
}

impl Actor for WsConn {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        WS_CLIENT_MAP.insert(self.device_id, ctx.address());
        self.start_heartbeat(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        WS_CLIENT_MAP.remove(&self.device_id);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let remote_addr = &self.remote_addr_untrusted;
        match msg {
            Ok(ws::Message::Text(payload)) => self.handle_message_data(ctx, payload.into_bytes()),
            Ok(ws::Message::Binary(payload)) => self.handle_message_data(ctx, payload),
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                warn!(%remote_addr, "Closed websocket with reason: {reason:?}");
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => (),
            Err(e) => {
                error!(%remote_addr, "Protocol error: {e}");
                ctx.stop();
            }
        }
    }
}
