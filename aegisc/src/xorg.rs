use anyhow::{bail, Result};
use tracing::{debug, trace};

use sysinfo::{ProcessExt, SystemExt};

pub fn find_xorg_cmdline_auth() -> Result<String> {
    let s = sysinfo::System::new_all();
    for (pid, process) in s.processes().iter().filter(|(_, p)| p.name() == "Xorg") {
        trace!(
            "Found candidate Xorg process {} {}",
            pid,
            process.cmd().join(" ")
        );
        let mut auth_is_next = false;
        for arg in process.cmd() {
            if auth_is_next {
                return Ok(arg.to_owned());
            }
            if arg == "-auth" {
                auth_is_next = true;
            }
        }
    }
    bail!("Xorg process not found");
}

pub fn find_xauthority_path() -> Result<String> {
    if let Ok(xauth) = find_xorg_cmdline_auth() {
        return Ok(xauth);
    }
    bail!("Peeking into user's homes for a .Xauthority file is not implemented...")
}

pub fn setup_xorg_env_vars() -> Result<()> {
    let xauth = find_xauthority_path()?;
    debug!(xauth = xauth.as_str(), "Found xauthority file");
    std::env::set_var("XAUTHORITY", xauth);
    std::env::set_var("DISPLAY", ":0");
    Ok(())
}
