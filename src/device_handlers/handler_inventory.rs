use actix_web::error::Error;
use actix_web::web::Bytes;
use std::future::Future;
use std::pin::Pin;

pub type DeviceHandlerFn = fn(Bytes) -> Pin<Box<dyn Future<Output = Result<Bytes, Error>>>>;

pub struct DeviceHandler {
    pub path: &'static str,
    pub handler: DeviceHandlerFn,
}

pub fn device_handler_iter() -> impl Iterator<Item = &'static DeviceHandler> {
    inventory::iter::<DeviceHandler>.into_iter()
}

inventory::collect!(DeviceHandler);
