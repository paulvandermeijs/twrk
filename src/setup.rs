use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn run(workdir: &Path, env: &[(&str, &str)], commands: &[String]) -> Result<()> {
    for cmd in commands {
        let mut child = Command::new("sh");
        child
            .arg("-c")
            .arg(cmd)
            .current_dir(workdir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        for (key, value) in env {
            child.env(key, value);
        }
        let status = child
            .status()
            .with_context(|| format!("failed to spawn setup command: {cmd}"))?;
        if !status.success() {
            bail!("setup command failed ({status}): {cmd}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn runs_single_command_in_workdir() {
        let dir = tempdir().unwrap();
        let cmds = vec!["touch marker".to_string()];
        run(dir.path(), &[], &cmds).unwrap();
        assert!(dir.path().join("marker").exists());
    }

    #[test]
    fn runs_multiple_commands_sequentially() {
        let dir = tempdir().unwrap();
        let cmds = vec![
            "echo first > a".to_string(),
            "echo second > b".to_string(),
        ];
        run(dir.path(), &[], &cmds).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("a")).unwrap().trim(), "first");
        assert_eq!(fs::read_to_string(dir.path().join("b")).unwrap().trim(), "second");
    }

    #[test]
    fn bails_on_first_non_zero_exit() {
        let dir = tempdir().unwrap();
        let cmds = vec![
            "touch before".to_string(),
            "exit 7".to_string(),
            "touch after".to_string(),
        ];
        let err = run(dir.path(), &[], &cmds).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("exit 7") || msg.contains("exit code: 7"), "got: {msg}");
        assert!(dir.path().join("before").exists());
        assert!(!dir.path().join("after").exists(), "should not run after failure");
    }

    #[test]
    fn env_vars_are_visible_to_commands() {
        let dir = tempdir().unwrap();
        let env = [("TWRK_REPO_ROOT", "/tmp/fake-root"), ("TWRK_CONFIG", "dev")];
        let cmds = vec!["printf '%s\\n%s' \"$TWRK_REPO_ROOT\" \"$TWRK_CONFIG\" > out".to_string()];
        run(dir.path(), &env, &cmds).unwrap();
        let out = fs::read_to_string(dir.path().join("out")).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["/tmp/fake-root", "dev"]);
    }

    #[test]
    fn empty_commands_is_noop() {
        let dir = tempdir().unwrap();
        run(dir.path(), &[], &[]).unwrap();
    }
}
