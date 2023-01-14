//! Device handlers are attached to a known device. Exposed through REST and Websocket.
mod handler_inventory;
pub use handler_inventory::{device_handler_iter, DeviceHandlerFn};

use aegisd_handler_macros::device_handler;
use aegislib::command::device::{
    DeviceEvent, EventLogLevel, StatusArg, StatusReply, StoreCameraPictureArg,
    StoreCameraPictureReply,
};

use crate::model::device::get_status;
use crate::model::events;
use crate::model::pics::DeviceCameraPicture;
use anyhow::Result;
use axum::body::Bytes;
use chrono::Utc;
use sqlx::PgConnection;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct DeviceId(pub i32);

#[device_handler("/status")]
pub async fn status(
    db: &mut PgConnection,
    dev_id: DeviceId,
    _args: StatusArg,
) -> Result<StatusReply> {
    Ok(get_status(db, dev_id.0).await?.into())
}

#[device_handler("/store_camera_picture")]
pub async fn store_camera_picture(
    db: &mut PgConnection,
    dev_id: DeviceId,
    args: StoreCameraPictureArg,
) -> Result<StoreCameraPictureReply> {
    let now = Utc::now().naive_utc();
    let pic_size_kb = args.jpeg_data.len() / 1024;
    DeviceCameraPicture {
        id: 0,
        dev_id: dev_id.0,
        created_at: now,
        jpeg_data: args.jpeg_data,
    }
    .insert(db)
    .await?;
    let _ = events::insert(
        db,
        dev_id.0,
        DeviceEvent {
            timestamp: now.timestamp() as u64,
            level: EventLogLevel::Info,
            message: format!("Camera picture uploaded ({pic_size_kb}kiB)"),
        },
    )
    .await;
    Ok(StoreCameraPictureReply {})
}

#[device_handler("/log_event")]
pub async fn log_event(db: &mut PgConnection, dev_id: DeviceId, event: DeviceEvent) -> Result<()> {
    events::insert(db, dev_id.0, event).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::error::Result;
    use crate::model::device::test::insert_test_device;
    use crate::server::make_test_server;
    use aegislib::crypto::randomized_signature;
    use axum::body::Bytes;
    use base64::prelude::*;
    use ed25519_dalek::Keypair;
    use http::{Request, Response, StatusCode};
    use hyper::Body;
    use sqlx::PgPool;
    use tower::Service;

    fn signed_request<T: Into<Bytes>>(url: &str, body: T, key: &Keypair) -> Request<Body> {
        let body = body.into();
        let sig = randomized_signature(key, url.as_bytes(), body.as_ref());
        let sig = BASE64_URL_SAFE_NO_PAD.encode(sig);
        Request::post(url)
            .header("Authorization", "Bearer ".to_string() + &sig)
            .body(body.into())
            .unwrap()
    }

    #[sqlx::test]
    async fn missing_auth_header(db: PgPool) -> Result<()> {
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        let conn = &mut db.acquire().await?;
        insert_test_device(conn, device_pk.clone(), "test".into()).await?;

        let mut server = make_test_server(db).await?;
        let req = Request::post(format!("/device/{device_pk}/status"))
            .body(Body::empty())
            .unwrap();
        let mut resp: Response<_> = server.app.call(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body = hyper::body::to_bytes(resp.body_mut()).await?;
        assert_eq!(body, b"Missing Authorization header"[..]);
        Ok(())
    }

    #[sqlx::test]
    async fn bad_auth_header(db: PgPool) -> Result<()> {
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        let conn = &mut db.acquire().await?;
        insert_test_device(conn, device_pk.clone(), "test".into()).await?;

        let mut server = make_test_server(db).await?;
        let bad_key = Keypair::generate(&mut rand::thread_rng());
        let req = signed_request(&format!("/device/{device_pk}/status"), Vec::new(), &bad_key);
        let mut resp: Response<_> = server.app.call(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body = hyper::body::to_bytes(resp.body_mut()).await?;
        assert_eq!(body, b"Invalid signature"[..]);
        Ok(())
    }

    #[sqlx::test]
    async fn good_auth_header(db: PgPool) -> Result<()> {
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        let conn = &mut db.acquire().await?;
        insert_test_device(conn, device_pk.clone(), "test".into()).await?;

        let mut server = make_test_server(db).await?;
        let req = signed_request(
            &format!("/device/{device_pk}/status"),
            Vec::new(),
            &device_key,
        );
        let resp: Response<_> = server.app.call(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }
}
