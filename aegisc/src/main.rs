mod client;
mod config;
mod device_key;
mod lock;
mod module;
mod run_as;
mod webcam;

use crate::config::Config;
use aegislib::command::server::ServerCommand;
use anyhow::Result;
use clap::Arg;
use nix::unistd::{getpid, ROOT};
use tokio::sync::mpsc::channel;
use tracing::{error, info, trace, warn};
use tracing_subscriber::EnvFilter;

fn check_privs_and_module() {
    if !nix::unistd::geteuid().is_root() {
        warn!("We are not running as root!")
    } else if !nix::unistd::getuid().is_root() {
        // We are not root, but we're suid root. Elevate.
        info!("Running as setuid root. Strange, but continuing happily.");
        nix::unistd::setuid(ROOT).expect("Failed to setuid(0), but we have euid 0!");
    }

    if module::is_running() {
        // If there is a usermode helper running and it isn't us, we should exit to let it run
        let self_pid = getpid().as_raw();
        if let Ok(umh_pid) = module::read_umh_pid() {
            if umh_pid == self_pid {
                info!("Running as the kernel module's usermode helper, everything is nominal =]")
            } else if umh_pid != 0 {
                error!(
                    "A usermode helper is already running (pid {}), exiting",
                    umh_pid
                );
                std::process::exit(1);
            } else {
                warn!("The kernel module appears loaded but no usermode helper is running! Continuing...")
            }
        } else {
            warn!("Failed to read usermode helper pid! Continuing happily...")
        }
    } else if !module::is_running() {
        warn!("Kernel module does not appear to be loaded, trying modprobe");
        if let Err(e) = module::try_load() {
            error!("Could not load kernel module ({}), continuing anyways", e);
        } else {
            info!("Successfully loaded module, exiting to let the aegisc usermode helper instance run");
            std::process::exit(0);
        }
    }
}

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

    check_privs_and_module();

    let (event_tx, mut event_rx) = channel(1);
    let dev_key = device_key::get_or_create_keys(config.device_key_path.as_ref())?;
    let mut client = client::connect(config, dev_key, event_tx).await?;
    tracing::info!("Connected to server websocket");

    lock::apply_status(client.status().await?).await;

    while let Some(event) = event_rx.recv().await {
        trace!("Received server event: {:?}", event);
        match event {
            ServerCommand::StatusUpdate(status) => lock::apply_status(status).await,
        }
    }

    Ok(())
}
