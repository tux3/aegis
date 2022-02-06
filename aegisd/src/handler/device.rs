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
use crate::model::pics::InsertDeviceCameraPicture;
use actix_web::web::Bytes;
use anyhow::Result;
use chrono::Utc;
use ormx::Insert;
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
    InsertDeviceCameraPicture {
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
            message: format!("Camera picture uploaded ({}kiB)", pic_size_kb),
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
