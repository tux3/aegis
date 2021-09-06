//! Device handlers are attached to a known device. Exposed through REST and Websocket.
mod handler_inventory;
use handler_inventory::DeviceHandler;
pub use handler_inventory::{device_handler_iter, DeviceHandlerFn};

use aegislib::command::device::{StatusArg, StatusReply};
use device_handler_macro::device_handler;

use actix_web::error::Error;
use actix_web::web::Bytes;

#[device_handler("/status")]
pub async fn status(_args: StatusArg) -> Result<StatusReply, Error> {
    Ok(StatusReply {
        vt_locked: false,
        ssh_locked: false,
    })
}
