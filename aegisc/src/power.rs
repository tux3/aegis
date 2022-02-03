use crate::run_as::run_as_root;
use aegislib::command::server::PowerCommand;
use nix::sys::reboot::{reboot, RebootMode};
use nix::unistd::sync;
use tracing::error;

pub async fn apply_command(cmd: PowerCommand) {
    sync();
    let _ = match cmd {
        PowerCommand::Poweroff => reboot(RebootMode::RB_POWER_OFF),
        PowerCommand::Reboot => reboot(RebootMode::RB_AUTOBOOT),
    };
    error!("Reboot syscall failed, trying systemctl...");

    let _ = match cmd {
        PowerCommand::Poweroff => run_as_root(vec!["systemctl", "poweroff"]),
        PowerCommand::Reboot => run_as_root(vec!["systemctl", "reboot"]),
    };
}
