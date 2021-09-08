//! Device handlers are attached to a known device. Exposed through REST and Websocket.
mod handler_inventory;
pub use handler_inventory::{device_handler_iter, DeviceHandlerFn};

use aegisd_handler_macros::device_handler;
use aegislib::command::device::{StatusArg, StatusReply};

use actix_web::web::Bytes;
use anyhow::Result;
use sqlx::PgPool;

#[device_handler("/status")]
pub async fn status(_db: PgPool, _args: StatusArg) -> Result<StatusReply> {
    Ok(StatusReply {
        vt_locked: false,
        ssh_locked: false,
    })
}
