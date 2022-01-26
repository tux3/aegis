mod client;
mod config;
mod device_key;

use crate::config::Config;
use anyhow::Result;
use clap::Arg;
use tokio::sync::mpsc::channel;
use tracing::trace;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,actix_server=warn")
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

    let (event_tx, mut event_rx) = channel(1);
    let dev_key = device_key::get_or_create_keys(config.device_key_path.as_ref())?;
    let _client = client::connect(config, dev_key, event_tx).await?;
    tracing::info!("Connected to server websocket");

    while let Some(event) = event_rx.recv().await {
        trace!("Received server event: {:?}", event);
        // TODO: Handle events
    }

    Ok(())
}
