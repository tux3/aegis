//! Device handlers are attached to a known device. Exposed through REST and Websocket.
mod handler_inventory;
pub use handler_inventory::{device_handler_iter, DeviceHandlerFn};

use aegisd_handler_macros::device_handler;
use aegislib::command::device::{StatusArg, StatusReply};

use crate::model::device::get_status;
use actix_web::web::Bytes;
use anyhow::Result;
use sqlx::PgConnection;

#[derive(Copy, Clone)]
pub struct DeviceId(pub i32);

#[device_handler("/status")]
pub async fn status(
    db: &mut PgConnection,
    dev_id: DeviceId,
    _args: StatusArg,
) -> Result<StatusReply> {
    Ok(get_status(db, dev_id.0).await?.into())
}
