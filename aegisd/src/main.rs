mod config;
mod error;
mod handler;
mod middleware;
mod model;
mod ws;

use crate::handler::admin::admin_handler_iter;
use crate::handler::device::device_handler_iter;
use crate::handler::root::{health, register, websocket};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::Arg;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[actix_web::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,actix_server=warn,sqlx::query=warn")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = clap::Command::new("aegisd")
        .about("Server-side daemon for Aegis client protection")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .takes_value(true)
                .required(true)
                .help("Path to the config file"),
        )
        .get_matches();
    let config_path = args.value_of_os("config").unwrap();
    let config = config::Config::from_file(config_path);

    info!(
        db_host = &*config.db_host,
        db_name = &*config.db_name,
        db_user = &*config.db_user,
        "Connecting to database..."
    );
    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_conn)
        .connect(&format!(
            "postgres://{}:{}@{}/{}",
            config.db_user, config.db_pass, config.db_host, config.db_name
        ))
        .await?;
    info!("Running migrations...");
    sqlx::migrate!().run(&pool).await?;
    info!("Migration done");

    let app_config = Data::new(config.clone());
    let app_fn = move || {
        let app = App::new()
            .app_data(pool.clone())
            .app_data(app_config.clone())
            .service(websocket)
            .service(register)
            .service(health);

        let mut admin_scope = web::scope("/admin").wrap(middleware::AdminReqTransform);
        for handler in admin_handler_iter() {
            admin_scope = admin_scope.route(handler.path, web::post().to(handler.http_handler));
        }
        let app = app.service(admin_scope);

        let mut device_scope =
            web::scope("/device/{device_pk}").wrap(middleware::DeviceReqTransform);
        for handler in device_handler_iter() {
            device_scope = device_scope.route(handler.path, web::post().to(handler.http_handler));
        }
        let app = app.service(device_scope);
        app.wrap(Logger::default())
    };

    let server = HttpServer::new(app_fn).bind(("0.0.0.0", config.port))?;

    info!(port = config.port, "Server ready");
    server.run().await?;
    Ok(())
}
