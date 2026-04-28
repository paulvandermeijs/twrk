#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn resolve(input: &str) -> Result<PathBuf> {
    let expanded = expand_tilde(input);
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .context("could not read current directory")?
            .join(expanded)
    };
    canonicalise(&absolute)
}

fn expand_tilde(input: &str) -> PathBuf {
    if input == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(input));
    }
    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(input)
}

fn canonicalise(p: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(p).with_context(|| format!("could not resolve path {}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolves_absolute_path() {
        let dir = tempdir().unwrap();
        let resolved = resolve(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(resolved, dir.path().canonicalize().unwrap());
    }

    #[test]
    fn resolves_relative_dot() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let resolved = resolve(".").unwrap();
        assert_eq!(resolved, dir.path().canonicalize().unwrap());
    }
}
