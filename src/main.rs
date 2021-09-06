mod config;
mod device_handlers;
mod middleware;
mod root_handlers;
mod ws;

use crate::device_handlers::device_handler_iter;
use crate::root_handlers::{health, websocket};
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::Arg;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[actix_web::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    sodiumoxide::init().expect("Failed to initialize the crypto library");

    let args = clap::App::new("aegisd")
        .about("Server-side daemon for Aegis client protection")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .takes_value(true)
                .required(true)
                .about("Path to the config file"),
        )
        .get_matches();
    let config_path = args.value_of_os("config").unwrap();
    let config = config::Config::from_file(config_path);

    let app_fn = || {
        let app = App::new().service(websocket).service(health);

        let mut device_scope =
            web::scope("/device/{device_pk}").wrap(middleware::DeviceReqTransform);
        for handler in device_handler_iter() {
            device_scope = device_scope.route(handler.path, web::post().to(handler.handler));
        }
        app.service(device_scope)
    };

    let server = HttpServer::new(app_fn).bind(("0.0.0.0", config.port))?;

    info!(port = config.port, "Server ready");
    server.run().await?;
    Ok(())
}
