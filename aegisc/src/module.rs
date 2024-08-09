use crate::run_as::run_as_root;
use aegislib::client::DeviceClient;
use aegislib::command::device::{DeviceEvent, EventLogLevel};
use anyhow::{bail, Result};
use chrono::{DateTime, NaiveDateTime};
use nix::libc::pid_t;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use tracing::{debug, error, info};

pub fn is_running() -> bool {
    Path::exists("/sys/aegisk".as_ref())
}

pub fn read_umh_pid() -> Result<pid_t> {
    let pid: i32 = std::fs::read_to_string("/sys/aegisk/umh_pid")?
        .trim_end()
        .parse()?;
    Ok(pid as pid_t)
}

pub fn try_load() -> Result<()> {
    let out = run_as_root(vec!["modprobe", "aegisk"])?;
    if !out.status.success() {
        if !out.stdout.is_empty() {
            error!(
                "Stdout: {}",
                String::from_utf8_lossy(&out.stdout).trim_end()
            );
        }
        if !out.stderr.is_empty() {
            error!(
                "Stderr: {}",
                String::from_utf8_lossy(&out.stderr).trim_end()
            );
        }
        bail!("command returned {}", out.status);
    } else {
        debug!("Kernel module inserted, waiting for sysfs node to appear");
        for _ in 0..4 {
            if is_running() {
                info!("Kernel module loaded and running");
                return Ok(());
            }
            sleep(Duration::from_millis(250))
        }
        bail!("timed out waiting for sysfs node")
    }
}

pub struct InsertTime {
    pub insert_time: NaiveDateTime,
    pub boot_to_insert_delay: Duration,
}

pub fn get_insert_time() -> Result<InsertTime> {
    let data = std::fs::read_to_string("/sys/aegisk/insert_time")?;

    if let Some((Ok(l), Ok(r))) = data
        .trim_end()
        .split_once(' ')
        .map(|(l, r)| (l.parse::<u64>(), r.parse::<u64>()))
    {
        Ok(InsertTime {
            insert_time: DateTime::from_timestamp(
                (l / 1_000_000_000) as i64,
                (l % 1_000_000_000) as u32,
            )
            .unwrap()
            .naive_utc(),
            boot_to_insert_delay: Duration::from_millis(r / 1_000_000),
        })
    } else {
        bail!("Failed to parse module insert_time")
    }
}

pub async fn log_insert_time(client: &mut DeviceClient) {
    match get_insert_time() {
        Ok(time) => {
            let boot_to_insert_delay = humantime::format_duration(time.boot_to_insert_delay);
            if let Err(e) = client
                .log_event(DeviceEvent {
                    timestamp: time.insert_time.and_utc().timestamp() as u64,
                    level: EventLogLevel::Info,
                    message: format!("Module inserted ({boot_to_insert_delay} since boot)"),
                })
                .await
            {
                error!("Failed to log module insert_time: {e}")
            }
        }
        Err(e) => error!("Failed to read module insert_time sysfs file: {e}"),
    }
}
