//! Admin handlers are authenticated. They are reachable only by REST, not by websocket.

mod handler_inventory;
pub use handler_inventory::admin_handler_iter;

use crate::handler::device::DeviceId;
use crate::model::device::*;
use crate::model::{events, pics};
use crate::ws::ws_for_device;
use aegisd_handler_macros::admin_handler;
use aegislib::command::admin::{
    PendingDevice, RegisteredDevice, SendPowerCommandArg, SetStatusArg, StoredCameraPicture,
};
use aegislib::command::device::{DeviceEvent, EventLogLevel, StatusReply};
use aegislib::command::server::{ServerCommand, StatusUpdate};
use anyhow::{bail, Result};
use axum::body::Bytes;
use chrono::Utc;
use sqlx::PgConnection;
use tracing::warn;

#[admin_handler("/list_pending_devices")]
pub async fn list_pending_devices(db: &mut PgConnection) -> Result<Vec<PendingDevice>> {
    Ok(list_pending(db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[admin_handler("/delete_pending_device")]
pub async fn delete_pending_device(db: &mut PgConnection, name: String) -> Result<()> {
    delete_pending(db, &name).await?;
    Ok(())
}

#[admin_handler("/confirm_pending_device")]
pub async fn confirm_pending_device(db: &mut PgConnection, name: String) -> Result<()> {
    confirm_pending(db, &name).await?;
    let dev_id = get_dev_id_by_name(db, &name).await?;
    let _ = events::insert(
        db,
        dev_id,
        DeviceEvent {
            timestamp: Utc::now().timestamp() as u64,
            level: EventLogLevel::Info,
            message: "Device confirmed".into(),
        },
    )
    .await;
    Ok(())
}

#[admin_handler("/list_registered_devices")]
pub async fn list_registered_devices(db: &mut PgConnection) -> Result<Vec<RegisteredDevice>> {
    Ok(list_registered(db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[admin_handler("/delete_registered_device")]
pub async fn delete_registered_device(db: &mut PgConnection, name: String) -> Result<()> {
    delete_registered(db, &name).await?;
    Ok(())
}

#[admin_handler("/set_status")]
pub async fn set_status(db: &mut PgConnection, arg: SetStatusArg) -> Result<StatusReply> {
    let dev_id = get_dev_id_by_name(db, &arg.dev_name).await?;
    let status: StatusReply =
        update_status(db, dev_id, arg.vt_locked, arg.ssh_locked, arg.draw_decoy)
            .await?
            .into();
    if !arg.is_no_op() {
        let _ = events::insert(
            db,
            dev_id,
            DeviceEvent {
                timestamp: Utc::now().timestamp() as u64,
                level: EventLogLevel::Info,
                message: format!("Status updated: {status:?}"),
            },
        )
        .await;
    }
    if let Some(ws) = ws_for_device(DeviceId(dev_id)) {
        let status_update = StatusUpdate {
            ssh_locked: status.ssh_locked,
            vt_locked: status.vt_locked,
            draw_decoy: status.draw_decoy,
        };
        ws.send(status_update.into()).await.unwrap_or_else(|e| {
            warn!("Failed to send status update to websocket for device {dev_id}: {e}",);
        });
    }

    Ok(status)
}

#[admin_handler("/get_device_camera_pictures")]
pub async fn get_device_camera_pictures(
    db: &mut PgConnection,
    dev_name: String,
) -> Result<Vec<StoredCameraPicture>> {
    let dev_id = get_dev_id_by_name(db, &dev_name).await?;
    let pics = pics::get_for_device(db, dev_id).await?;
    tracing::info!("Sending {} device camera pictures", pics.len());
    Ok(pics.into_iter().map(Into::into).collect())
}

#[admin_handler("/delete_device_camera_pictures")]
pub async fn delete_device_camera_pictures(db: &mut PgConnection, dev_name: String) -> Result<()> {
    let dev_id = get_dev_id_by_name(db, &dev_name).await?;
    pics::delete_for_device(db, dev_id).await?;
    let _ = events::insert(
        db,
        dev_id,
        DeviceEvent {
            timestamp: Utc::now().timestamp() as u64,
            level: EventLogLevel::Debug,
            message: "Deleted stored camera pictures".into(),
        },
    )
    .await;
    Ok(())
}

#[admin_handler("/send_power_command")]
pub async fn send_power_command(db: &mut PgConnection, arg: SendPowerCommandArg) -> Result<()> {
    let dev_id = get_dev_id_by_name(db, &arg.dev_name).await?;
    let ws = match ws_for_device(DeviceId(dev_id)) {
        Some(ws) => ws,
        None => bail!("Device is not connected"),
    };

    ws.send(ServerCommand::PowerCommand(arg.command))
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to send power command to websocket for device {dev_id}: {e}",);
        });
    let _ = events::insert(
        db,
        dev_id,
        DeviceEvent {
            timestamp: Utc::now().timestamp() as u64,
            level: EventLogLevel::Info,
            message: format!("Sent power command: {:?}", arg.command),
        },
    )
    .await;

    Ok(())
}

#[admin_handler("/get_device_events")]
pub async fn get_device_events(
    db: &mut PgConnection,
    dev_name: String,
) -> Result<Vec<DeviceEvent>> {
    let dev_id = get_dev_id_by_name(db, &dev_name).await?;
    let events = events::get_for_device(db, dev_id).await?;
    tracing::info!("Sending {} device events", events.len());
    Ok(events)
}

#[admin_handler("/delete_device_events")]
pub async fn delete_device_events(db: &mut PgConnection, dev_name: String) -> Result<()> {
    let dev_id = get_dev_id_by_name(db, &dev_name).await?;
    events::delete_for_device(db, dev_id).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::error::Result;
    use crate::model::device;
    use crate::model::device::test::{insert_test_device, insert_test_pending_device};
    use crate::server::{make_test_server, TestServer};
    use aegislib::command::admin::{PendingDevice, RegisteredDevice, SetStatusArg};
    use aegislib::crypto::randomized_signature;
    use anyhow::anyhow;
    use axum::body::Bytes;
    use axum::response::Response;
    use base64::prelude::*;
    use ed25519_dalek::Keypair;
    use http::{Request, StatusCode};
    use hyper::Body;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use sqlx::PgPool;
    use tower_service::Service;

    fn signed_request(url: &str, body: Bytes, key: &Keypair) -> Request<Body> {
        let sig = randomized_signature(key, url.as_bytes(), body.as_ref());
        let sig = BASE64_URL_SAFE_NO_PAD.encode(sig);
        Request::post(url)
            .header("Authorization", "Bearer ".to_string() + &sig)
            .body(body.into())
            .unwrap()
    }

    async fn raw_request<T: Into<Bytes>>(
        server: &mut TestServer,
        url: &str,
        body: T,
    ) -> Result<Response> {
        let req = signed_request(url, body.into(), &server.root_key);
        Ok(server.app.call(req).await?)
    }

    async fn request<T: Serialize, U: DeserializeOwned>(
        server: &mut TestServer,
        url: &str,
        body: T,
    ) -> Result<U> {
        let body = Bytes::from(bincode::serialize(&body).unwrap());
        let mut resp = raw_request(server, url, body).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        let resp_data = hyper::body::to_bytes(resp.body_mut()).await?;
        Ok(bincode::deserialize_from(resp_data.as_ref())
            .map_err(|e| anyhow!("Failed to deserialize: {e}"))?)
    }

    #[sqlx::test]
    async fn missing_auth_header(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db).await?;
        let req = Request::post("/admin/list_pending_devices")
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
        let mut server = make_test_server(db).await?;
        let bad_key = Keypair::generate(&mut rand::thread_rng());
        let req = signed_request("/admin/list_pending_devices", Bytes::new(), &bad_key);
        let mut resp = server.app.call(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body = hyper::body::to_bytes(resp.body_mut()).await?;
        assert_eq!(body, b"Invalid signature"[..]);
        Ok(())
    }

    #[sqlx::test]
    async fn good_auth_header(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db).await?;
        let resp = raw_request(&mut server, "/admin/list_pending_devices", vec![]).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[sqlx::test]
    async fn list_pending(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db.clone()).await?;
        let devs: Vec<PendingDevice> =
            request(&mut server, "/admin/list_pending_devices", ()).await?;
        assert!(devs.is_empty());

        let conn = &mut db.acquire().await?;
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        insert_test_pending_device(conn, device_pk.clone(), "test".into()).await?;

        let devs: Vec<PendingDevice> =
            request(&mut server, "/admin/list_pending_devices", ()).await?;
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0].name, "test");
        assert_eq!(devs[0].pubkey, device_pk);
        Ok(())
    }

    #[sqlx::test]
    async fn confirm_pending(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db.clone()).await?;
        let conn = &mut db.acquire().await?;
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        insert_test_pending_device(conn, device_pk.clone(), "test".into()).await?;

        request(&mut server, "/admin/confirm_pending_device", "test").await?;
        let pending = device::list_pending(conn).await?;
        assert!(pending.is_empty());
        let devs = device::list_registered(conn).await?;
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0].name, "test");
        assert_eq!(devs[0].pubkey, device_pk);
        assert!(!devs[0].pending);
        Ok(())
    }

    #[sqlx::test]
    async fn delete_pending(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db.clone()).await?;
        let conn = &mut db.acquire().await?;
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        insert_test_pending_device(conn, device_pk.clone(), "test".into()).await?;

        request(&mut server, "/admin/delete_pending_device", "test").await?;
        let pending = device::list_pending(conn).await?;
        assert!(pending.is_empty());
        let devs = device::list_registered(conn).await?;
        assert!(devs.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn list_registered(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db.clone()).await?;
        let devs: Vec<RegisteredDevice> =
            request(&mut server, "/admin/list_registered_devices", ()).await?;
        assert!(devs.is_empty());

        let conn = &mut db.acquire().await?;
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        insert_test_device(conn, device_pk.clone(), "test".into()).await?;

        let devs: Vec<RegisteredDevice> =
            request(&mut server, "/admin/list_registered_devices", ()).await?;
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0].name, "test");
        assert_eq!(devs[0].pubkey, device_pk);
        Ok(())
    }

    #[sqlx::test]
    async fn delete_registered(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db.clone()).await?;
        let conn = &mut db.acquire().await?;
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        insert_test_device(conn, device_pk.clone(), "test".into()).await?;

        request(&mut server, "/admin/delete_registered_device", "test").await?;
        assert!(device::list_registered(conn).await?.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn set_status(db: PgPool) -> Result<()> {
        let mut server = make_test_server(db.clone()).await?;
        let conn = &mut db.acquire().await?;
        let device_key = Keypair::generate(&mut rand::thread_rng());
        let device_pk = BASE64_URL_SAFE_NO_PAD.encode(device_key.public.as_ref());
        insert_test_device(conn, device_pk.clone(), "test".into()).await?;

        request(
            &mut server,
            "/admin/set_status",
            SetStatusArg {
                dev_name: "test".to_string(),
                vt_locked: Some(true),
                ssh_locked: Some(false),
                draw_decoy: None,
            },
        )
        .await?;
        let id = device::get_dev_id_by_name(conn, "test").await?;
        let status = device::get_status(conn, id).await?;
        assert!(status.vt_locked);
        assert!(!status.ssh_locked);
        assert!(!status.draw_decoy);
        Ok(())
    }
}
