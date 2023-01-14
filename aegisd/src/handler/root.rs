//! Root handlers are unauthenticated. They are reachable only by REST, not by websocket.

use crate::error::{bail, Result};
use crate::handler::device::DeviceId;
use crate::model::device;
use crate::model::device::{count_pending, PendingDevice};
use crate::ws::WsConn;
use axum::extract::{ConnectInfo, Path, State, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use base64::prelude::*;
use chrono::Utc;
use ed25519_dalek::PublicKey;
use futures::StreamExt;
use http::Request;
use hyper::{Body, StatusCode};
use sqlx::{Error as SqlxError, PgPool};
use std::net::SocketAddr;
use tracing::{debug, error};

pub async fn health() -> &'static str {
    "ok"
}

pub async fn websocket_upgrade(
    State(db): State<PgPool>,
    Path(device_pk): Path<String>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    ws_upgrade: WebSocketUpgrade,
) -> Result<Response> {
    let device_pk = BASE64_URL_SAFE_NO_PAD.decode(device_pk).ok();
    let device_pk = match device_pk.and_then(|pk| PublicKey::from_bytes(&pk).ok()) {
        Some(pk) => pk,
        None => bail!(StatusCode::BAD_REQUEST, "Invalid device_id"),
    };

    let conn = &mut db.acquire().await?;
    let device_id = match device::get_dev_id_by_pk(conn, &device_pk).await {
        Err(e) => return Err((StatusCode::FORBIDDEN, format!("Device not found: {e}")).into()),
        Ok(id) => DeviceId(id),
    };
    Ok(ws_upgrade.on_upgrade(move |ws| async move {
        debug!(%remote_addr, "Device websocket connection established");
        let ws_conn = WsConn::new(db, device_pk, device_id, remote_addr.to_string());
        if let Err(e) = ws_conn.handle(ws).await {
            error!("Error handling ws client {}: {}", remote_addr, e)
        }
    }))
}

pub async fn register(
    State(db): State<PgPool>,
    Path((device_pk, name)): Path<(String, String)>,
    request: Request<Body>,
) -> Result<Response> {
    if request.into_body().next().await.is_some() {
        bail!(StatusCode::BAD_REQUEST, "Unexpected body");
    }
    let device_pk = BASE64_URL_SAFE_NO_PAD.decode(device_pk).ok();
    let device_pk = match device_pk.and_then(|pk| PublicKey::from_bytes(&pk).ok()) {
        Some(pk) => pk,
        None => bail!(StatusCode::BAD_REQUEST, "Invalid device_id"),
    };

    let mut conn = db.acquire().await?;

    if count_pending(&mut conn).await? >= 3 {
        bail!(StatusCode::BAD_REQUEST, "Too many pending devices");
    }

    let pubkey_str = BASE64_URL_SAFE_NO_PAD.encode(device_pk);
    let insert_result = PendingDevice {
        created_at: Utc::now().naive_utc(),
        name,
        pubkey: pubkey_str,
    }
    .insert(&mut conn)
    .await;
    let unique_violation_code = "23505"; // Per https://www.postgresql.org/docs/current/errcodes-appendix.html
    match insert_result {
        Err(SqlxError::Database(e)) if e.code().as_deref() == Some(unique_violation_code) => {
            return Ok(StatusCode::CONFLICT.into_response())
        }
        result => result.map(|_| ())?,
    };
    Ok(().into_response())
}

#[cfg(test)]
mod test {
    use crate::error::Result;
    use crate::model::device::list_pending;
    use crate::server::make_test_server;
    use aegislib::crypto::random_sign_keypair;
    use base64::prelude::*;
    use http::{Request, Response, StatusCode};
    use hyper::Body;
    use sqlx::PgPool;
    use tower::Service;

    #[sqlx::test]
    async fn health(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db).await?;
        let req = Request::get("/health").body(Body::empty()).unwrap();
        let mut resp: Response<_> = server.app.call(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(resp.body_mut()).await?;
        assert_eq!(body, "ok".as_bytes());
        Ok(())
    }

    #[sqlx::test]
    async fn register_unexpected_body(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db).await?;
        let dev_pk = BASE64_URL_SAFE_NO_PAD.encode(random_sign_keypair().public);
        let req = Request::post(format!("/register/{dev_pk}/name/test"))
            .body(b"body"[..].into())
            .unwrap();
        let resp: Response<_> = server.app.call(req).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[sqlx::test]
    async fn register(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db.clone()).await?;
        let dev_pk = BASE64_URL_SAFE_NO_PAD.encode(random_sign_keypair().public);
        let req = Request::post(format!("/register/{dev_pk}/name/test"))
            .body(Body::empty())
            .unwrap();
        let mut resp: Response<_> = server.app.call(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = hyper::body::to_bytes(resp.body_mut()).await?;
        assert!(body.is_empty());

        let conn = &mut db.acquire().await?;
        let pending = list_pending(conn).await?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].name, "test");
        Ok(())
    }
}
