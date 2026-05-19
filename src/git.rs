use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

#[must_use]
pub fn repo_root(path: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let common_dir = PathBuf::from(s.trim());
    common_dir.parent().map(Path::to_path_buf)
}

pub fn ensure_worktree(repo_root: &Path, name: &str) -> Result<(PathBuf, bool)> {
    let target = repo_root.join(".worktrees").join(name);
    if target.is_dir() {
        return Ok((target, false));
    }
    std::fs::create_dir_all(target.parent().unwrap())
        .with_context(|| format!("could not create {}", target.parent().unwrap().display()))?;
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["worktree", "add", "-b", name])
        .arg(&target)
        .output()
        .context("failed to spawn `git worktree add`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git worktree add failed (exit status {}): {}",
            output.status,
            stderr.trim()
        );
    }
    Ok((target, true))
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
    fn repo_root_returns_main_repo_from_inside_a_worktree() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        let (wt, _) = ensure_worktree(dir.path(), "feat-y").unwrap();
        let root = repo_root(&wt).expect("repo_root should resolve from inside a worktree");
        assert_eq!(
            root.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn ensure_worktree_reports_created_then_not_created() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        let (p1, created1) = ensure_worktree(dir.path(), "feat-x").unwrap();
        assert!(p1.is_dir());
        assert!(created1, "first call should report created=true");
        let (p2, created2) = ensure_worktree(dir.path(), "feat-x").unwrap();
        assert_eq!(p1, p2);
        assert!(!created2, "second call should report created=false");
    }
}
