use crate::error::Error;
use crate::handler::device::DeviceId;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::Request;
use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;

type PinBoxFut<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type DeviceHttpHandlerFn = fn(State<PgPool>, Request<Body>) -> PinBoxFut<Result<Bytes, Error>>;
pub type DeviceHandlerFn = fn(PgPool, DeviceId, Bytes) -> PinBoxFut<Result<Bytes, Error>>;

pub struct DeviceHandler {
    pub path: &'static str,
    pub http_handler: DeviceHttpHandlerFn,
    pub handler: DeviceHandlerFn,
}

pub fn device_handler_iter() -> impl Iterator<Item = &'static DeviceHandler> {
    inventory::iter::<DeviceHandler>.into_iter()
}

inventory::collect!(DeviceHandler);
