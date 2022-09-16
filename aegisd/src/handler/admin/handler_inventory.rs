use crate::error::Error;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::Request;
use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;

type PinBoxFut<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type AdminHttpHandlerFn = fn(State<PgPool>, Request<Body>) -> PinBoxFut<Result<Bytes, Error>>;

pub struct AdminHandler {
    pub path: &'static str,
    pub http_handler: AdminHttpHandlerFn,
}

pub fn admin_handler_iter() -> impl Iterator<Item = &'static AdminHandler> {
    inventory::iter::<AdminHandler>.into_iter()
}

inventory::collect!(AdminHandler);
