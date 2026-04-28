use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn run_in(cwd: &Path, command: &str) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let status = Command::new(&shell)
        .current_dir(cwd)
        .args(["-c", command])
        .status()
        .with_context(|| format!("failed to spawn {shell}"))?;
    if !status.success() {
        bail!("command exited with status {status}");
    }
    Ok(())
}
