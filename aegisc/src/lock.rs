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

    // TODO: Apply vt_lock status
}
