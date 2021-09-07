use actix_web::error::Error;
use actix_web::web::Bytes;
use actix_web::HttpRequest;
use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;

type PinBoxFut<T> = Pin<Box<dyn Future<Output = T>>>;
pub type DeviceHttpHandlerFn = fn(HttpRequest, Bytes) -> PinBoxFut<Result<Bytes, Error>>;
pub type DeviceHandlerFn = fn(PgPool, Bytes) -> PinBoxFut<Result<Bytes, Error>>;

pub struct DeviceHandler {
    pub path: &'static str,
    pub http_handler: DeviceHttpHandlerFn,
    pub handler: DeviceHandlerFn,
}

pub fn device_handler_iter() -> impl Iterator<Item = &'static DeviceHandler> {
    inventory::iter::<DeviceHandler>.into_iter()
}

inventory::collect!(DeviceHandler);
