use actix_web::error::Error;
use actix_web::web::Bytes;
use actix_web::HttpRequest;
use std::future::Future;
use std::pin::Pin;

type PinBoxFut<T> = Pin<Box<dyn Future<Output = T>>>;
pub type AdminHttpHandlerFn = fn(HttpRequest, Bytes) -> PinBoxFut<Result<Bytes, Error>>;

pub struct AdminHandler {
    pub path: &'static str,
    pub http_handler: AdminHttpHandlerFn,
}

pub fn admin_handler_iter() -> impl Iterator<Item = &'static AdminHandler> {
    inventory::iter::<AdminHandler>.into_iter()
}

inventory::collect!(AdminHandler);
