#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::{Context, Result};

const ENV_VAR: &str = "TWRK_WORKSPACE";

pub fn roots() -> Result<Vec<PathBuf>> {
    if let Ok(value) = std::env::var(ENV_VAR) {
        return Ok(parse_roots(&value));
    }
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(vec![home.join("Workspace")])
}

pub fn list_projects(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(read) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.push(path);
            }
        }
    }
    out.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    out
}

fn parse_roots(value: &str) -> Vec<PathBuf> {
    value
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(expand_tilde)
        .collect()
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_roots_splits_lines_and_trims() {
        let input = "/a\n  /b  \n\n/c\n";
        let r = parse_roots(input);
        assert_eq!(
            r,
            vec![PathBuf::from("/a"), PathBuf::from("/b"), PathBuf::from("/c")]
        );
    }

    #[test]
    fn list_projects_returns_sorted_dirs_from_each_root() {
        let a = tempdir().unwrap();
        let b = tempdir().unwrap();
        fs::create_dir(a.path().join("zeta")).unwrap();
        fs::create_dir(a.path().join("alpha")).unwrap();
        fs::write(a.path().join("ignored.txt"), "x").unwrap();
        fs::create_dir(b.path().join("gamma")).unwrap();

        let projects = list_projects(&[a.path().into(), b.path().into()]);
        let names: Vec<_> = projects
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["alpha", "gamma", "zeta"]);
    }
}
