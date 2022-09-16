use crate::error::{bail, Error};
use crate::handler::device::DeviceId;
use crate::model::device::get_dev_id_by_pk;
use aegislib::crypto::check_signature;
use anyhow::anyhow;
use axum::extract::{ConnectInfo, FromRequestParts, OriginalUri, Path};
use axum::response::{IntoResponse, Response};
use futures::future::Future;
use futures::TryFutureExt;
use http::{Request, StatusCode};
use hyper::Body;
use sqlx::PgPool;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

#[derive(Clone)]
pub struct DeviceAuthLayer {
    pub db: PgPool,
}

impl DeviceAuthLayer {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

impl<S> Layer<S> for DeviceAuthLayer {
    type Service = DeviceAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            db: self.db.clone(),
        }
    }
}

#[derive(Clone)]
pub struct DeviceAuthMiddleware<S> {
    inner: S,
    db: PgPool,
}

impl<S> Service<Request<Body>> for DeviceAuthMiddleware<S>
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
        let db = self.db.clone();
        // We must only use the service that was poll_ready, and store back the clone
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(
            async move {
                let (mut parts, body) = req.into_parts();
                let params: HashMap<String, String> =
                    match Path::from_request_parts(&mut parts, &()).await {
                        Ok(Path(path)) => path,
                        Err(e) => bail!(
                            StatusCode::BAD_REQUEST,
                            format!("Failed to extract path parameters: {e}")
                        ),
                    };
                let device_pk = params
                    .get("device_pk")
                    .and_then(|pk| base64::decode_config(pk, base64::URL_SAFE_NO_PAD).ok())
                    .and_then(|pk| ed25519_dalek::PublicKey::from_bytes(&pk).ok());
                let device_pk = match device_pk {
                    Some(device_pk) => device_pk.to_owned(),
                    None => bail!(StatusCode::BAD_REQUEST, "Invalid device_pk"),
                };
                let mut conn = db
                    .acquire()
                    .await
                    .map_err(|e| anyhow!("Database error: {e}"))?;
                let dev_id = match get_dev_id_by_pk(&mut conn, &device_pk).await {
                    Err(e) => bail!(StatusCode::FORBIDDEN, format!("Device not found: {e}")),
                    Ok(id) => id,
                };
                let _ = parts.extensions.insert(DeviceId(dev_id));

                let auth_header = match parts.headers.get("Authorization") {
                    Some(auth) => auth,
                    None => bail!(StatusCode::FORBIDDEN, "Missing Authorization header"),
                };
                let bearer = auth_header
                    .as_bytes()
                    .strip_prefix(b"Bearer ")
                    .and_then(|bearer| base64::decode_config(bearer, base64::URL_SAFE_NO_PAD).ok());
                let randomized_signature = match bearer {
                    Some(bearer) => bearer,
                    _ => bail!(StatusCode::FORBIDDEN, "Invalid Authorization header"),
                };

                let body_bytes = hyper::body::to_bytes(body).await.map_err(|e| {
                    Error::Response(StatusCode::BAD_REQUEST, format!("Failed to read body: {e}"))
                })?;

                let uri = &parts.extensions.get::<OriginalUri>().unwrap().0;
                if !check_signature(
                    &device_pk,
                    &randomized_signature,
                    uri.path().as_bytes(),
                    body_bytes.as_ref(),
                ) {
                    let remote_addr = match parts.extensions.get::<ConnectInfo<SocketAddr>>() {
                        Some(ConnectInfo(addr)) => addr.to_owned(),
                        None => SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0),
                    };
                    warn!(%remote_addr, "Received forged signature from client!");
                    bail!(StatusCode::FORBIDDEN, "Invalid signature");
                }

                let req = Request::from_parts(parts, body_bytes.into());
                inner.call(req).await.map_err(Error::from)
            }
            .or_else(|e| async { Ok(e.into_response()) }),
        )
    }
}
