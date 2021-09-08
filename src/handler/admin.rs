//! Admin handlers are authenticated. They are reachable only by REST, not by websocket.

mod handler_inventory;
pub use handler_inventory::admin_handler_iter;

use crate::model::device::list_pending;
use actix_web::web::Bytes;
use aegisd_handler_macros::admin_handler;
use aegislib::command::admin::PendingDevice;
use anyhow::Result;
use sqlx::PgConnection;

#[admin_handler("/list_pending_devices")]
pub async fn list_pending_devices(db: &mut PgConnection, _args: ()) -> Result<Vec<PendingDevice>> {
    Ok(list_pending(db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}
