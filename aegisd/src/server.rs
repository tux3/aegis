use crate::config::Config;
use crate::handler::admin::admin_handler_iter;
use crate::handler::device::device_handler_iter;
use crate::handler::root::{health, register, websocket_upgrade};
use crate::middleware::{AdminAuthLayer, DeviceAuthLayer};
use anyhow::Result;
use axum::routing::{get, post, Router};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, TraceLayer};
use tracing::{info, Level};

pub async fn make_router(db: PgPool, config: &Config) -> Result<Router> {
    let mut app = Router::new()
        .route("/health", get(health))
        .route("/ws/:device_pk", get(websocket_upgrade))
        .route("/register/:device_pk/name/:name", post(register))
        .with_state::<()>(db.clone());

    let admin_router = admin_handler_iter()
        .fold(Router::new(), |router, handler| {
            router.route(handler.path, post(handler.http_handler))
        })
        .layer(AdminAuthLayer::new(config.clone()))
        .with_state(db.clone());
    app = app.nest("/admin", admin_router);

    let device_router = device_handler_iter()
        .fold(Router::new(), |router, handler| {
            router.route(handler.path, post(handler.http_handler))
        })
        .layer(DeviceAuthLayer::new(db.clone()))
        .with_state(db);
    app = app.nest("/device/:device_pk", device_router);

    app = app.layer(
        ServiceBuilder::new()
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_request(DefaultOnRequest::new().level(Level::INFO)),
            )
            .layer(CompressionLayer::new()),
    );

    Ok(app)
}

pub async fn run_server(db: PgPool, config: &Config) -> Result<()> {
    let app = make_router(db, config).await?;
    let fut = axum::Server::bind(&SocketAddr::new([0, 0, 0, 0].into(), config.port))
        .serve(app.into_make_service_with_connect_info::<SocketAddr>());
    info!(port = config.port, "Server ready");
    fut.await?;
    Ok(())
}

#[cfg(test)]
pub struct TestServer {
    pub app: Router,
    pub config: Config,
    pub root_key: ed25519_dalek::Keypair,
}

#[cfg(test)]
pub async fn make_test_server(db: PgPool) -> Result<TestServer> {
    let root_key = ed25519_dalek::Keypair::generate(&mut rand::thread_rng());
    let config = Config::test_config(root_key.public);
    let app = make_router(db, &config).await?;
    Ok(TestServer {
        app,
        config,
        root_key,
    })
}
