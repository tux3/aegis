use crate::config::Config;
use actix_service::{Service, Transform};
use actix_web::error::ErrorForbidden;
use actix_web::web::{BytesMut, Data};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use aegislib::crypto::check_signature;
use futures::future::{ok, Future, Ready};
use futures::stream::StreamExt;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use tracing::warn;

pub struct AdminReqTransform;

impl<S: 'static> Transform<S, ServiceRequest> for AdminReqTransform
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Transform = AdminReqMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AdminReqMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct AdminReqMiddleware<S> {
    // This is special: We need this to avoid lifetime issues.
    service: Rc<RefCell<S>>,
}

impl<S: 'static> Service<ServiceRequest> for AdminReqMiddleware<S>
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

        Box::pin(async move {
            let config = req
                .app_data::<Data<Config>>()
                .expect("missing config app data");
            let root_sig_pk = config.root_public_signature_key;

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
            let route = req.path();

            if !check_signature(
                &root_sig_pk,
                &randomized_signature,
                route.as_bytes(),
                body.as_ref(),
            ) {
                let remote_addr = req
                    .connection_info()
                    .realip_remote_addr()
                    .unwrap()
                    .to_owned();
                warn!(%remote_addr, "Received forged signature from admin client!");
                return Err(ErrorForbidden("Invalid signature"));
            }

            let (_, mut payload) = actix_http::h1::Payload::create(true);
            payload.unread_data(body.into());
            req.set_payload(payload.into());

            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}
