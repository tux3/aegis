//! Root handlers are unauthenticated. They are reachable only by REST, not by websocket.

use actix_web::{get, Error, Responder, HttpResponse, HttpRequest};
use actix_web::web::Payload;
use crate::ws::WsConn;

#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

#[get("/ws")]
pub async fn websocket(req: HttpRequest, stream: Payload) -> Result<HttpResponse, Error> {
    let remote_addr = req
        .connection_info()
        .realip_remote_addr() // Not trusted!
        .unwrap() // Only None for unit tests
        .to_owned();
    let ws = WsConn::new(remote_addr);

    // Upgrade to a websocket
    let resp = actix_web_actors::ws::start(ws, &req, stream)?;
    Ok(resp)
}
