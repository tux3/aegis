//! Admin handlers are authenticated. They are reachable only by REST, not by websocket.

mod handler_inventory;
pub use handler_inventory::admin_handler_iter;

use crate::model::device::*;
use actix_web::web::Bytes;
use aegisd_handler_macros::admin_handler;
use aegislib::command::admin::{PendingDevice, RegisteredDevice, SetStatusArg};
use aegislib::command::device::StatusReply;
use anyhow::Result;
use sqlx::PgConnection;

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
    Ok(update_status(db, dev_id, arg.vt_locked, arg.ssh_locked)
        .await?
        .into())
}