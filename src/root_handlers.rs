//! Root handlers are unauthenticated. They are reachable only by REST, not by websocket.

use crate::ws::WsConn;
use actix_web::error::ErrorBadRequest;
use actix_web::web::{Path, Payload};
use actix_web::{get, Error, HttpRequest, HttpResponse, Responder};
use sodiumoxide::base64;
use sodiumoxide::base64::Variant::UrlSafeNoPadding;
use sodiumoxide::crypto::sign::PublicKey;

#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

#[get("/ws/{device_pk}")]
pub async fn websocket(
    req: HttpRequest,
    stream: Payload,
    path: Path<(String,)>,
) -> Result<HttpResponse, Error> {
    let remote_addr = req
        .connection_info()
        .realip_remote_addr() // Not trusted!
        .unwrap() // Only None for unit tests
        .to_owned();
    let device_pk = path.into_inner().0;
    let device_pk = base64::decode(device_pk, UrlSafeNoPadding).ok();
    let device_pk = match device_pk.and_then(|pk| PublicKey::from_slice(&pk)) {
        Some(pk) => pk,
        None => return Err(ErrorBadRequest("Invalid device_id")),
    };
    let ws = WsConn::new(device_pk, remote_addr);

    // Upgrade to a websocket
    let resp = actix_web_actors::ws::start(ws, &req, stream)?;
    Ok(resp)
}
