use crate::run_as::run_as_root;
use aegislib::command::server::PowerCommand;
use tracing::error;

pub async fn apply_command(cmd: PowerCommand) {
    // NOTE: We cannot simply call the reboot syscall as the umh
    // The reboot syscall waits for the uhm to exit, so we would deadlock...

    let arg_str = match cmd {
        PowerCommand::Reboot => "reboot",
        PowerCommand::Poweroff => "poweroff",
    };
    if let Err(e) = std::fs::write("/sys/aegisk/power", arg_str) {
        error!("Failed to write power control file, module may not be running ({e})");
    }

    // Fallback, just in case (perfectly OK if it races with the module)
    let _ = match cmd {
        PowerCommand::Poweroff => run_as_root(vec!["systemctl", "poweroff"]),
        PowerCommand::Reboot => run_as_root(vec!["systemctl", "reboot"]),
    };
}
