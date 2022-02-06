//! Admin handlers are authenticated. They are reachable only by REST, not by websocket.

mod handler_inventory;
pub use handler_inventory::admin_handler_iter;

use crate::handler::device::DeviceId;
use crate::model::device::*;
use crate::model::{events, pics};
use crate::ws::{ws_for_device, WsServerCommand};
use actix_web::web::Bytes;
use aegisd_handler_macros::admin_handler;
use aegislib::command::admin::{
    PendingDevice, RegisteredDevice, SendPowerCommandArg, SetStatusArg, StoredCameraPicture,
};
use aegislib::command::device::{DeviceEvent, EventLogLevel, StatusReply};
use aegislib::command::server::{ServerCommand, StatusUpdate};
use anyhow::{bail, Result};
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
            timestamp: Utc::now().naive_utc().timestamp() as u64,
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
                timestamp: Utc::now().naive_utc().timestamp() as u64,
                level: EventLogLevel::Info,
                message: format!("Status updated: {:?}", &status),
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
        ws.send(WsServerCommand::from(status_update))
            .await
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to send status update to websocket for device {}: {}",
                    dev_id, e
                );
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
            timestamp: Utc::now().naive_utc().timestamp() as u64,
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

    ws.send(WsServerCommand::from(ServerCommand::PowerCommand(
        arg.command,
    )))
    .await
    .unwrap_or_else(|e| {
        warn!(
            "Failed to send power command to websocket for device {}: {}",
            dev_id, e
        );
    });
    let _ = events::insert(
        db,
        dev_id,
        DeviceEvent {
            timestamp: Utc::now().naive_utc().timestamp() as u64,
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
