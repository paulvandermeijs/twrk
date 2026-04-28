use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub worktree: Option<bool>,
    pub layout: BTreeMap<String, Layout>,
}

pub type Layout = Vec<Window>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Window {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub split: Split,
    pub content: Vec<Element>,
}

/// A child of a `Window`. Today this is always a `Pane`. The enum is
/// `untagged` so future variants (e.g. nested groups) can be added without
/// breaking existing config files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Element {
    Pane(Pane),
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Split {
    #[default]
    Cols,
    Rows,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pane {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub command: String,
}

pub fn load_for(project_dir: &Path) -> Result<Config> {
    let mut chain: Vec<PathBuf> = Vec::new();
    let mut cur = Some(project_dir.to_path_buf());
    while let Some(dir) = cur {
        if let Some(file) = find_config_in(&dir) {
            chain.push(file);
        }
        cur = dir.parent().map(Path::to_path_buf);
    }
    chain.reverse(); // root-most first; project last (highest priority)

    let mut merged = serde_json::Value::Null;
    for file in chain {
        let value = read_as_value(&file)
            .with_context(|| format!("failed to read config {}", file.display()))?;
        merged = deep_merge(merged, value);
    }
    if merged.is_null() {
        return Ok(Config::default());
    }
    let cfg: Config = serde_json::from_value(merged).context("invalid merged config")?;
    Ok(cfg)
}

#[must_use]
pub fn resolve_layout(cfg: &Config, id: &str) -> Layout {
    if let Some(found) = cfg.layout.get(id) {
        return found.clone();
    }
    fallback_layout()
}

fn fallback_layout() -> Layout {
    vec![Window {
        name: Some("main".into()),
        split: Split::Cols,
        content: vec![],
    }]
}

fn find_config_in(dir: &Path) -> Option<PathBuf> {
    for name in [".twrk.toml", ".twrk.yaml", ".twrk.yml", ".twrk.json"] {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn read_as_value(path: &Path) -> Result<serde_json::Value> {
    let raw = std::fs::read_to_string(path)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    Ok(match ext {
        "toml" => toml::from_str(&raw)?,
        "yaml" | "yml" => serde_yml::from_str(&raw)?,
        "json" => serde_json::from_str(&raw)?,
        other => anyhow::bail!("unknown config extension: {other}"),
    })
}

fn deep_merge(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    use serde_json::Value::Object;
    match (base, overlay) {
        (Object(mut a), Object(b)) => {
            for (k, v) in b {
                let merged = match a.remove(&k) {
                    Some(existing) => deep_merge(existing, v),
                    None => v,
                };
                a.insert(k, merged);
            }
            Object(a)
        }
        (_, overlay) => overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_yaml() -> &'static str {
        r"
worktree: true
layout:
  default:
    - name: Project
      content:
        - name: Nu shell
          command: nu
  dev:
    - name: Dev
      content:
        - name: Claude
          command: claude -r
        - name: Editor
          command: hx
    - name: Server
      content:
        - command: npm run dev
    - name: Logs
      split: rows
      content:
        - name: State
          command: watch -n /var/logs/state.txt
        - name: Events
          command: tail -f /var/logs/my.log
"
    }

    fn pane(name: Option<&str>, cmd: &str) -> Element {
        Element::Pane(Pane {
            name: name.map(str::to_string),
            command: cmd.to_string(),
        })
    }

    fn expected() -> Config {
        let mut layout = BTreeMap::new();
        layout.insert(
            "default".into(),
            vec![Window {
                name: Some("Project".into()),
                split: Split::Cols,
                content: vec![pane(Some("Nu shell"), "nu")],
            }],
        );
        layout.insert(
            "dev".into(),
            vec![
                Window {
                    name: Some("Dev".into()),
                    split: Split::Cols,
                    content: vec![pane(Some("Claude"), "claude -r"), pane(Some("Editor"), "hx")],
                },
                Window {
                    name: Some("Server".into()),
                    split: Split::Cols,
                    content: vec![pane(None, "npm run dev")],
                },
                Window {
                    name: Some("Logs".into()),
                    split: Split::Rows,
                    content: vec![
                        pane(Some("State"), "watch -n /var/logs/state.txt"),
                        pane(Some("Events"), "tail -f /var/logs/my.log"),
                    ],
                },
            ],
        );
        Config {
            worktree: Some(true),
            layout,
        }
    }

    #[test]
    fn parses_yaml_fixture() {
        let cfg: Config = serde_yml::from_str(fixture_yaml()).unwrap();
        assert_eq!(cfg, expected());
    }

    #[test]
    fn parses_json_equivalent() {
        let json = serde_json::to_string(&expected()).unwrap();
        let cfg: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, expected());
    }

    #[test]
    fn parses_toml_equivalent() {
        let toml_str = toml::to_string(&expected()).unwrap();
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, expected());
    }

    #[test]
    fn split_defaults_to_cols() {
        let yaml = "layout:\n  default:\n    - content:\n        - command: ls\n";
        let cfg: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(cfg.layout["default"][0].split, Split::Cols);
    }

    #[test]
    fn deep_merge_overrides_scalars_and_merges_maps() {
        let a = serde_json::json!({ "worktree": false, "layout": { "default": [1] } });
        let b = serde_json::json!({ "worktree": true,  "layout": { "dev": [2] } });
        let m = deep_merge(a, b);
        assert_eq!(
            m,
            serde_json::json!({
                "worktree": true,
                "layout": { "default": [1], "dev": [2] }
            })
        );
    }

    #[test]
    fn deep_merge_arrays_overwrite() {
        let a = serde_json::json!({ "layout": { "default": [{"name": "old"}] } });
        let b = serde_json::json!({ "layout": { "default": [{"name": "new"}] } });
        let m = deep_merge(a, b);
        assert_eq!(m["layout"]["default"][0]["name"], "new");
        assert_eq!(m["layout"]["default"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn load_for_walks_up_and_lower_wins() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");
        std::fs::create_dir_all(&child).unwrap();

        std::fs::write(
            parent.join(".twrk.yaml"),
            "worktree: false\nlayout:\n  default:\n    - content:\n        - command: from-parent\n",
        )
        .unwrap();
        std::fs::write(
            child.join(".twrk.yaml"),
            "worktree: true\nlayout:\n  default:\n    - content:\n        - command: from-child\n",
        )
        .unwrap();

        let cfg = load_for(&child).unwrap();
        assert_eq!(cfg.worktree, Some(true));
        let Element::Pane(pane) = &cfg.layout["default"][0].content[0];
        assert_eq!(pane.command, "from-child");
    }

    #[test]
    fn load_for_prefers_toml_over_yaml_in_same_dir() {
        let root = tempfile::tempdir().unwrap();
        // toml has no `worktree` key
        std::fs::write(
            root.path().join(".twrk.toml"),
            "[layout]\n",
        )
        .unwrap();
        // yaml would set worktree, but yaml should never be read
        std::fs::write(
            root.path().join(".twrk.yaml"),
            "worktree: true\n",
        )
        .unwrap();
        let cfg = load_for(root.path()).unwrap();
        // If yaml had been read, this would be Some(true).
        assert_eq!(cfg.worktree, None);
    }

    #[test]
    fn load_for_returns_default_when_no_files() {
        let root = tempfile::tempdir().unwrap();
        let cfg = load_for(root.path()).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn resolve_layout_returns_named_layout() {
        let cfg = expected();
        let layout = resolve_layout(&cfg, "dev");
        assert_eq!(layout.len(), 3);
    }

    #[test]
    fn resolve_layout_falls_back_when_missing() {
        let cfg = Config::default();
        let layout = resolve_layout(&cfg, "default");
        assert_eq!(layout.len(), 1);
        assert!(layout[0].content.is_empty());
    }
}
