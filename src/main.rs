mod config;

use anyhow::Result;
use clap::Arg;
use tracing_subscriber::EnvFilter;

#[tokio::main]
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

    let args = clap::App::new("aegisc")
        .about("Client-side Aegis daemon")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .takes_value(true)
                .required(true)
                .help("Path to the config file"),
        )
        .get_matches();
    let config_path = args
        .value_of_os("config")
        .map(Into::into)
        .unwrap_or_else(config::default_path);
    let config = config::Config::from_file(config_path);
    Ok(())
}
