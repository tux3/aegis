use anyhow::{bail, Result};
use std::process::{Command, Output};
use tracing::debug;

pub fn run_as_root(mut cmdline: Vec<&str>) -> Result<Output> {
    if !nix::unistd::geteuid().is_root() {
        cmdline.insert(0, "sudo");
        cmdline.insert(1, "-n");
    }
    debug!("Running command: {}", cmdline.join(" "));

    let cmd = cmdline.remove(0);
    let args = cmdline;
    match Command::new(cmd).args(args).output() {
        Err(e) => bail!(e),
        Ok(out) => Ok(out),
    }
}
