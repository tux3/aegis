//! Device handlers are attached to a known device. Exposed through REST and Websocket.
mod handler_inventory;
use handler_inventory::DeviceHandler;
pub use handler_inventory::{device_handler_iter, DeviceHandlerFn};

use device_handler_macro::device_handler;

use actix_web::error::Error;
use actix_web::error::ErrorBadRequest;
use actix_web::web::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct DeviceStatus {}

#[device_handler("/status")]
pub async fn status(body: Bytes) -> Result<Bytes, Error> {
    if !body.is_empty() {
        return Err(ErrorBadRequest("Unexpected body"));
    }

    let device_status = DeviceStatus {};
    Ok(bincode::serialize(&device_status).unwrap().into())
}
