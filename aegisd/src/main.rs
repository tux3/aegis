mod config;
mod error;
mod handler;
mod middleware;
mod model;
mod server;
mod ws;

use clap::Arg;
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
                .allow_invalid_utf8(true)
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

    server::run_server(pool, &config).await?;
    Ok(())
}
