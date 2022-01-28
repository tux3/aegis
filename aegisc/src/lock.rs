use crate::run_as::run_as_root;
use aegislib::command::server::StatusUpdate;
use anyhow::Result;
use nix::libc::ioctl;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use tracing::{error, info, warn};

fn set_vt_lock_ioctl(lock: bool) -> Result<()> {
    let tty = File::open("/dev/tty0")?; // Requires root (or tty group membership)

    const REGULAR_VT_NUM: u64 = 7; // Cheap assumption... this is fallback best effort code
    const LOCK_TARGET_VT_NUM: u64 = 25; // Probably unused arbitrary VT
    const VT_ACTIVATE: u64 = 0x5606;
    const VT_LOCKSWITCH: u64 = 0x560B;
    const VT_UNLOCKSWITCH: u64 = 0x560C;

    let target = if lock {
        LOCK_TARGET_VT_NUM
    } else {
        REGULAR_VT_NUM
    };
    unsafe {
        if !lock {
            ioctl(tty.as_raw_fd(), VT_UNLOCKSWITCH, 0);
        }
        ioctl(tty.as_raw_fd(), VT_ACTIVATE, target);
        if lock {
            ioctl(tty.as_raw_fd(), VT_LOCKSWITCH, 0);
        }
    }
    Ok(())
}

fn set_vt_lock(lock: bool) -> Result<()> {
    let bool_str = if lock { "1" } else { "0" };
    if let Err(e) = std::fs::write("/sys/aegisk/lock_vt", bool_str) {
        warn!(
            "Failed to set vt_lock file, module may not be running ({})",
            e
        );
        return set_vt_lock_ioctl(lock);
    }
    Ok(())
}

pub fn apply_status(status: impl Into<StatusUpdate>) {
    let status = status.into();
    info!("Applying device status: {:?}", status);
    if status.ssh_locked {
        if let Err(e) = run_as_root(vec!["systemctl", "stop", "ssh"]) {
            error!("Failed to lock SSH: {}", e)
        }
    } else if let Err(e) = run_as_root(vec!["systemctl", "start", "ssh"]) {
        error!("Failed to unlock SSH: {}", e)
    }

    if let Err(e) = set_vt_lock(status.vt_locked) {
        error!("Failed to set vt_lock ({})", e);
    }
}
