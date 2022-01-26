use crate::run_as::run_as_root;
use anyhow::{bail, Result};
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
