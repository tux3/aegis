mod client;
mod config;
mod device_key;

use crate::config::Config;
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
                .help("Path to the config file"),
        )
        .get_matches();
    let config_path = args
        .value_of_os("config")
        .map(Into::into)
        .unwrap_or_else(config::default_path);
    let config = &Config::from_file(config_path).unwrap_or_else(|_| Config::default());
    tracing::info!(
        device_name = config.device_name.as_str(),
        server_addr = config.server_addr.as_str(),
        use_tls = config.use_tls,
        "Loaded config"
    );

    let dev_key = device_key::get_or_create_keys(config.device_key_path.as_ref())?;
    let client = client::connect(config, dev_key).await?;
    tracing::info!("Connected to server websocket");

    Ok(())
}
