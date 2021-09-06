mod cmd;
mod config;

use anyhow::{Context, Result};
use clap::{clap_app, AppSettings};

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    sodiumoxide::init().expect("Failed to initialize the crypto library");

    let args = clap_app!(aegiscli =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (about: env!("CARGO_PKG_DESCRIPTION"))
        (@arg config: -c --config +takes_value "Path to the config file")
        (@subcommand "gen-device-key" =>
            (about: "Generate a random device key file")
            (@arg output: +required "The destination file")
        )
    )
    .setting(AppSettings::ArgRequiredElseHelp)
    .get_matches();

    let config_path = args
        .value_of_os("config")
        .map(|os| os.into())
        .unwrap_or_else(config::default_path);
    let config = config::Config::from_file(config_path);

    match args.subcommand().unwrap() {
        ("gen-device-key", sub_args) => cmd::gen_device_key(&config, sub_args).await,
        _ => unreachable!(),
    }
    .with_context(|| format!("\r{} failed", args.subcommand_name().unwrap()))
}
