mod config;

use anyhow::Result;
use clap::AppSettings;

fn main() -> Result<()> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = clap::clap_app!(aegiscli =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (about: env!("CARGO_PKG_DESCRIPTION"))
        (@arg config: -c --config +takes_value "Path to the config file")
    )
    .setting(AppSettings::ArgRequiredElseHelp)
    .get_matches();

    let config_path = args
        .value_of_os("config")
        .map(|os| os.into())
        .unwrap_or_else(config::default_path);
    let _config = config::Config::from_file(config_path);

    Ok(())
}
