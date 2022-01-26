use crate::run_as::run_as_root;
use aegislib::command::server::StatusUpdate;
use tracing::{error, info};

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

    if status.vt_locked {
        if let Err(e) = std::fs::write("/sys/aegisk/lock_vt", "1") {
            error!("Failed to set vt_lock file ({})", e);
        }
    } else if let Err(e) = std::fs::write("/sys/aegisk/lock_vt", "0") {
        error!("Failed to unset vt_lock file ({})", e);
    }
    // TODO: Apply vt_lock status by hand if module is not working
}
