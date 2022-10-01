mod cmd;
mod config;

use aegislib::client::{AdminClient, DeviceClient};
use aegislib::crypto::sign_keypair_from_file;
use anyhow::{Context, Result};
use clap::{arg, command, value_parser, Command};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = command!()
        .arg(
            arg!(-c --config <path> "Path to the config file")
                .value_parser(value_parser!(PathBuf))
                .required(false),
        )
        .arg(arg!(use_rest: --rest "Use REST API instead of websockets").required(false))
        .subcommand_required(true)
        .subcommand(
            Command::new("gen-device-key")
                .about("Generate a random device key file")
                .arg(arg!(<output> "The destination file").value_parser(value_parser!(PathBuf))),
        )
        .subcommand(
            Command::new("derive-root-key-file")
                .about("Generate a root key file from a password")
                .arg(arg!(<output> "The destination file").value_parser(value_parser!(PathBuf)))
                .arg(arg!([password] "The password for the new root key")),
        )
        .subcommand(
            Command::new("derive-root-pubkey")
                .about("Generate a root public signature key from a password")
                .arg(arg!([password] "The password for the new root key")),
        )
        .subcommand(
            Command::new("register")
                .about("Register as a device pending validation by an admin")
                .arg(arg!(<key> "The device private key file").value_parser(value_parser!(PathBuf)))
                .arg(arg!(<name> "The device's name")),
        )
        .subcommand(
            Command::new("admin")
                .about("Send control request using the admin root keys")
                .arg(arg!(<key> "The admin root key file").value_parser(value_parser!(PathBuf)))
                .subcommand(
                    Command::new("list-pending")
                        .about("List registered devices pending validation"),
                )
                .subcommand(
                    Command::new("delete-pending")
                        .about("Delete a device pending validation")
                        .arg(arg!(<name> "The device's name")),
                )
                .subcommand(
                    Command::new("confirm-pending")
                        .about("Confirm a device pending validation")
                        .arg(arg!(<name> "The device's name")),
                )
                .subcommand(Command::new("list-device").about("List valid registered devices"))
                .subcommand(
                    Command::new("delete-device")
                        .about("Delete a valid registered device")
                        .arg(arg!(<name> "The device's name")),
                )
                .subcommand(
                    Command::new("set-status")
                        .about("Update status for a registered device")
                        .arg(arg!(<name> "The device's name"))
                        .arg(
                            arg!(--"vt-lock" <value> "Lock the system onto a blank TTY")
                                .required(false),
                        )
                        .arg(arg!(--"ssh-lock" <value> "Disable new SSH logins").required(false))
                        .arg(
                            arg!(--"draw-decoy" <value> "Use decoy TTY framebuffer")
                                .required(false),
                        ),
                ),
        )
        .subcommand(
            Command::new("device")
                .about("Send requests as if running on a device")
                .arg(arg!(<key> "The device private key file").value_parser(value_parser!(PathBuf)))
                .subcommand(
                    Command::new("status").about("Get the expected status for this device"),
                ),
        )
        .get_matches();

    let config_path = args
        .get_one::<PathBuf>("config")
        .map(|p| p.to_owned())
        .unwrap_or_else(config::default_path);
    let mut config = config::Config::from_file(config_path);
    if args.get_flag("use_rest") {
        config.use_rest = true;
    }
    let config = &config;

    if !cfg!(debug_assertions) && !config.use_tls {
        tracing::warn!("TLS should be enabled in release configurations!");
    }

    match args.subcommand().unwrap() {
        ("gen-device-key", sub_args) => cmd::gen_device_key(config, sub_args).await,
        ("derive-root-key-file", sub_args) => cmd::derive_root_key_file(config, sub_args).await,
        ("derive-root-pubkey", sub_args) => cmd::derive_root_pubkey(config, sub_args).await,
        ("register", sub_args) => cmd::register(config, sub_args).await,
        ("admin", admin_args) => {
            let root_keys = std::fs::read(admin_args.get_one::<PathBuf>("key").unwrap())?;
            let root_keys = bincode::deserialize(&root_keys)?;
            let client = AdminClient::new(&config.into(), &root_keys).await?;
            match admin_args.subcommand().unwrap() {
                ("list-pending", sub_args) => {
                    cmd::admin::list_pending(config, client, sub_args).await
                }
                ("delete-pending", sub_args) => {
                    cmd::admin::delete_pending(config, client, sub_args).await
                }
                ("confirm-pending", sub_args) => {
                    cmd::admin::confirm_pending(config, client, sub_args).await
                }
                ("list-device", sub_args) => {
                    cmd::admin::list_registered(config, client, sub_args).await
                }
                ("delete-device", sub_args) => {
                    cmd::admin::delete_registered(config, client, sub_args).await
                }
                ("set-status", sub_args) => cmd::admin::set_status(config, client, sub_args).await,
                _ => unreachable!(),
            }
        }
        ("device", dev_args) => {
            let dev_key = sign_keypair_from_file(dev_args.get_one::<PathBuf>("key").unwrap())?;
            let client = DeviceClient::new(&config.into(), dev_key, None)
                .await
                .map_err(|(_, e)| e)?;
            match dev_args.subcommand().unwrap() {
                ("status", sub_args) => cmd::device::status(config, client, sub_args).await,
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
    .with_context(|| format!("\r{} command failed!", args.subcommand_name().unwrap()))
}
