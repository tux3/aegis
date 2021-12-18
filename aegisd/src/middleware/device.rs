use crate::handler::device::DeviceId;
use crate::model::device::get_dev_id_by_pk;
use actix_service::{Service, Transform};
use actix_web::error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError};
use actix_web::web::BytesMut;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use aegislib::crypto::check_signature;
use futures::future::{ok, Future, Ready};
use futures::stream::StreamExt;
use sqlx::PgPool;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use tracing::warn;

pub struct DeviceReqTransform;

impl<S: 'static> Transform<S, ServiceRequest> for DeviceReqTransform
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Transform = DeviceReqMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(DeviceReqMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct DeviceReqMiddleware<S> {
    // This is special: We need this to avoid lifetime issues.
    service: Rc<RefCell<S>>,
}

impl<S: 'static> Service<ServiceRequest> for DeviceReqMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    #[allow(clippy::type_complexity)] // Actix has complex types! I'm doing my best here :(
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    actix_service::forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let db = req.app_data::<PgPool>().cloned().unwrap();

        Box::pin(async move {
            let device_pk = req
                .match_info()
                .get("device_pk")
                .and_then(|pk| base64::decode_config(pk, base64::URL_SAFE_NO_PAD).ok())
                .and_then(|pk| ed25519_dalek::PublicKey::from_bytes(&pk).ok());
            let device_pk = match device_pk {
                Some(device_pk) => device_pk.to_owned(),
                None => return Err(ErrorBadRequest("Invalid device_pk")),
            };
            let mut conn = db
                .acquire()
                .await
                .map_err(|e| ErrorInternalServerError(format!("Database error: {}", e)))?;
            let dev_id = match get_dev_id_by_pk(&mut conn, &device_pk).await {
                Err(e) => return Err(ErrorForbidden(format!("Device not found: {}", e))),
                Ok(id) => id,
            };
            req.extensions_mut().insert(DeviceId(dev_id)).unwrap();

            let auth_header = match req.headers().get("Authorization") {
                Some(auth) => auth,
                None => return Err(ErrorForbidden("Missing Authorization header")),
            };
            let bearer = auth_header
                .as_bytes()
                .strip_prefix(b"Bearer ")
                .and_then(|bearer| base64::decode_config(bearer, base64::URL_SAFE_NO_PAD).ok());
            let randomized_signature = match bearer {
                Some(bearer) => bearer,
                _ => return Err(ErrorForbidden("Invalid Authorization header")),
            };

            let mut body = BytesMut::new();
            let mut stream = req.take_payload();
            while let Some(chunk) = stream.next().await {
                body.extend_from_slice(&chunk?);
            }
            let route = req.path().as_bytes();

            if !check_signature(&device_pk, &randomized_signature, route, body.as_ref()) {
                let remote_addr = req
                    .connection_info()
                    .realip_remote_addr()
                    .unwrap()
                    .to_owned();
                warn!(%remote_addr, "Received forged signature from client!");
                return Err(ErrorForbidden("Invalid signature"));
            }

            let mut payload = actix_http::h1::Payload::empty();
            payload.unread_data(body.into());
            req.set_payload(payload.into());

            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}
