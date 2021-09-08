//! Root handlers are unauthenticated. They are reachable only by REST, not by websocket.

use crate::error::{bail, Error};
use crate::model::device;
use crate::model::device::{count_pending, PendingDevice};
use crate::ws::WsConn;
use actix_web::error::ErrorForbidden;
use actix_web::web::{Path, Payload};
use actix_web::{get, post, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use futures::StreamExt;
use ormx::Insert;
use sodiumoxide::base64;
use sodiumoxide::base64::Variant::UrlSafeNoPadding;
use sodiumoxide::crypto::sign::PublicKey;
use sqlx::PgPool;
use tracing::debug;

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
    let db = req.app_data::<PgPool>().cloned().unwrap();
    let remote_addr = req
        .connection_info()
        .realip_remote_addr() // Not trusted!
        .unwrap() // Only None for unit tests
        .to_owned();
    let device_pk = path.into_inner().0;
    let device_pk = base64::decode(device_pk, UrlSafeNoPadding).ok();
    let device_pk = match device_pk.and_then(|pk| PublicKey::from_slice(&pk)) {
        Some(pk) => pk,
        None => bail!("Invalid device_id"),
    };

    let conn = &mut db.acquire().await?;
    if !device::is_key_registered(conn, &device_pk).await? {
        return Err(ErrorForbidden("Device not registered").into());
    }

    let ws = WsConn::new(db, device_pk, remote_addr.clone());

    // Upgrade to a websocket
    let resp = actix_web_actors::ws::start(ws, &req, stream)?;

    debug!(%remote_addr, "Device websocket connection established");
    Ok(resp)
}

#[post("/register/{device_pk}/name/{name}")]
pub async fn register(
    req: HttpRequest,
    mut body: Payload,
    path: Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    if body.next().await.is_some() {
        bail!("Unexpected body");
    }
    let (device_pk, name) = path.into_inner();
    let device_pk = base64::decode(device_pk, UrlSafeNoPadding).ok();
    let device_pk = match device_pk.and_then(|pk| PublicKey::from_slice(&pk)) {
        Some(pk) => pk,
        None => bail!("Invalid device_id"),
    };

    let db = req.app_data::<PgPool>().cloned().unwrap();
    let mut conn = db.acquire().await?;

    if count_pending(&mut *conn).await? >= 3 {
        bail!("Too many pending devices");
    }

    PendingDevice {
        created_at: Utc::now().naive_utc(),
        name,
        pubkey: device_pk.into(),
    }
    .insert(&mut *conn)
    .await?;
    Ok(HttpResponse::Ok().finish())
}
