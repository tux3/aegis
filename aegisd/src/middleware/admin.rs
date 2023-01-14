use crate::config::Config;
use crate::error::{bail, Error};
use aegislib::crypto::check_signature;
use axum::extract::{ConnectInfo, OriginalUri};
use axum::response::{IntoResponse, Response};
use base64::prelude::*;
use ed25519_dalek::PublicKey;
use futures::TryFutureExt;
use http::{Request, StatusCode};
use hyper::Body;
use std::convert::Infallible;
use std::fmt::Debug;
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

#[derive(Clone)]
pub struct AdminAuthLayer {
    pub config: Config,
}

impl AdminAuthLayer {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for AdminAuthLayer {
    type Service = AdminAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AdminAuthMiddleware {
            inner,
            root_pk: self.config.root_public_signature_key,
        }
    }
}

#[derive(Clone)]
pub struct AdminAuthMiddleware<S> {
    inner: S,
    root_pk: PublicKey,
}

impl<S> Service<Request<Body>> for AdminAuthMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Debug,
    Error: From<S::Error>,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(ctx).map_err(|e| panic!("{e:?}"))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let root_sig_pk = self.root_pk;
        // We must only use the service that was poll_ready, and store back the clone
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(
            async move {
                let auth_header = match req.headers().get("Authorization") {
                    Some(auth) => auth,
                    None => bail!(StatusCode::FORBIDDEN, "Missing Authorization header"),
                };
                let bearer = auth_header
                    .as_bytes()
                    .strip_prefix(b"Bearer ")
                    .and_then(|bearer| BASE64_URL_SAFE_NO_PAD.decode(bearer).ok());
                let randomized_signature = match bearer {
                    Some(bearer) => bearer,
                    _ => bail!(StatusCode::FORBIDDEN, "Invalid Authorization header"),
                };

                let (parts, body) = req.into_parts();
                let body_bytes = hyper::body::to_bytes(body).await.map_err(|e| {
                    Error::Response(StatusCode::BAD_REQUEST, format!("Failed to read body: {e}"))
                })?;

                let uri = &parts.extensions.get::<OriginalUri>().unwrap().0;
                if !check_signature(
                    &root_sig_pk,
                    &randomized_signature,
                    uri.path().as_bytes(),
                    body_bytes.as_ref(),
                ) {
                    let remote_addr = match parts.extensions.get::<ConnectInfo<SocketAddr>>() {
                        Some(ConnectInfo(addr)) => addr.to_owned(),
                        None => SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0),
                    };
                    warn!(%remote_addr, "Received forged signature from admin client!");
                    bail!(StatusCode::FORBIDDEN, "Invalid signature");
                }

                let req = Request::from_parts(parts, body_bytes.into());
                inner.call(req).await.map_err(Error::from)
            }
            .or_else(|e| async { Ok(e.into_response()) }),
        )
    }
}
