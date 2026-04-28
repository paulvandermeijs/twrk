use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

#[must_use]
pub fn repo_root(path: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    Some(PathBuf::from(s.trim()))
}

pub fn ensure_worktree(repo_root: &Path, name: &str) -> Result<PathBuf> {
    let target = repo_root.join(".worktrees").join(name);
    if target.is_dir() {
        return Ok(target);
    }
    std::fs::create_dir_all(target.parent().unwrap())
        .with_context(|| format!("could not create {}", target.parent().unwrap().display()))?;
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["worktree", "add", "-b", name])
        .arg(&target)
        .status()
        .context("failed to spawn `git worktree add`")?;
    if !status.success() {
        bail!("git worktree add failed (exit status {status})");
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn init_repo(dir: &Path) {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["init", "-q", "-b", "main"])
            .status()
            .unwrap();
        std::fs::write(dir.join("README.md"), "x").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["add", "."])
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-qm",
                "init",
            ])
            .status()
            .unwrap();
    }

    #[test]
    fn repo_root_returns_none_for_non_repo() {
        let dir = tempdir().unwrap();
        assert!(repo_root(dir.path()).is_none());
    }

    #[test]
    fn repo_root_returns_root_for_subdir() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let root = repo_root(&sub).unwrap();
        assert_eq!(
            root.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn ensure_worktree_creates_and_is_idempotent() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        let p1 = ensure_worktree(dir.path(), "feat-x").unwrap();
        assert!(p1.is_dir());
        let p2 = ensure_worktree(dir.path(), "feat-x").unwrap();
        assert_eq!(p1, p2);
    }
}
