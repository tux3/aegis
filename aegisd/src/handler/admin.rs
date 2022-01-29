//! Admin handlers are authenticated. They are reachable only by REST, not by websocket.

mod handler_inventory;
pub use handler_inventory::admin_handler_iter;

use crate::handler::device::DeviceId;
use crate::model::device::*;
use crate::ws::{ws_for_device, WsServerCommand};
use actix_web::web::Bytes;
use aegisd_handler_macros::admin_handler;
use aegislib::command::admin::{PendingDevice, RegisteredDevice, SetStatusArg};
use aegislib::command::device::StatusReply;
use aegislib::command::server::StatusUpdate;
use anyhow::Result;
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
