#![feature(result_option_inspect)]

mod client;
mod config;
mod device_key;
mod event;
mod lock;
mod module;
mod power;
mod run_as;
mod webcam;
mod xorg;

use crate::config::Config;
use crate::event::ClientEvent;
use crate::xorg::setup_xorg_env_vars;
use aegislib::client::DeviceClient;
use aegislib::command::device::{DeviceEvent, EventLogLevel};
use aegislib::command::server::ServerCommand;
use anyhow::Result;
use chrono::Utc;
use clap::Arg;
use nix::unistd::{getpid, ROOT};
use tokio::spawn;
use tokio::sync::mpsc::{channel, Receiver};
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

async fn handle_server_events(mut event_rx: Receiver<ServerCommand>) {
    while let Some(event) = event_rx.recv().await {
        trace!("Received server event: {:?}", event);
        match event {
            ServerCommand::StatusUpdate(status) => lock::apply_status(status).await,
            ServerCommand::PowerCommand(cmd) => power::apply_command(cmd).await,
        }
    }
    error!("Server event receiver closed, quitting immediately!");
    std::process::exit(1);
}

async fn handle_client_events(
    mut client: DeviceClient,
    mut client_event_rx: Receiver<ClientEvent>,
) {
    while let Some(event) = client_event_rx.recv().await {
        match event {
            ClientEvent::WebcamPicture(data) => {
                let size = data.len() as f32 / 1024.0;
                if let Err(e) = client.store_camera_picture(data).await {
                    error!("Failed to upload webcam picture: {}", e);
                } else {
                    info!("Successfully uploaded {:.1}kB camera picture!", size)
                }
            }
            ClientEvent::InputWhileLockedWithoutWebcam => {
                let _ = client
                    .log_event(DeviceEvent {
                        timestamp: Utc::now().timestamp() as u64,
                        level: EventLogLevel::Info,
                        message: "Detected input while locked (no webcam picture)".to_string(),
                    })
                    .await;
            }
        }
    }
    error!("Client event receiver closed, quitting immediately!");
    std::process::exit(1);
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
    if let Err(e) = setup_xorg_env_vars() {
        error!("Failed to setup Xorg env, screenshots may not work: {}", e);
    }

    let (event_tx, event_rx) = channel(1);
    let dev_key = device_key::get_or_create_keys(config.device_key_path.as_ref())?;
    let mut client = client::connect(config, dev_key, event_tx).await?;
    tracing::info!("Connected to server websocket");

    module::log_insert_time(&mut client).await;
    lock::apply_status(client.status().await?).await;
    spawn(handle_server_events(event_rx));

    let (client_event_tx, client_event_rx) = channel(1);
    lock::register_event_tx(client_event_tx).await;
    handle_client_events(client, client_event_rx).await;

    Ok(())
}
