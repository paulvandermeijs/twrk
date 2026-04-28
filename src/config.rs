use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub type Config = BTreeMap<String, Group>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Group {
    pub worktree: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
}

pub type Layout = Vec<Window>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Window {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub split: Split,
    #[serde(
        default,
        deserialize_with = "null_as_default",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub content: Vec<Element>,
}

fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::deserialize(d)?.unwrap_or_default())
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
    chain.reverse();

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
    cfg.get(id)
        .and_then(|g| g.layout.clone())
        .unwrap_or_else(fallback_layout)
}

#[must_use]
pub fn group_worktree(cfg: &Config, id: &str) -> Option<bool> {
    cfg.get(id).and_then(|g| g.worktree)
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
default:
  worktree: false
  layout:
    - name: Project
      content:
        - name: Nu shell
          command: nu
dev:
  worktree: true
  layout:
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
        let mut cfg = Config::new();
        cfg.insert(
            "default".into(),
            Group {
                worktree: Some(false),
                layout: Some(vec![Window {
                    name: Some("Project".into()),
                    split: Split::Cols,
                    content: vec![pane(Some("Nu shell"), "nu")],
                }]),
            },
        );
        cfg.insert(
            "dev".into(),
            Group {
                worktree: Some(true),
                layout: Some(vec![
                    Window {
                        name: Some("Dev".into()),
                        split: Split::Cols,
                        content: vec![
                            pane(Some("Claude"), "claude -r"),
                            pane(Some("Editor"), "hx"),
                        ],
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
                ]),
            },
        );
        cfg
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
        let yaml = "default:\n  layout:\n    - content:\n        - command: ls\n";
        let cfg: Config = serde_yml::from_str(yaml).unwrap();
        let layout = cfg["default"].layout.as_ref().unwrap();
        assert_eq!(layout[0].split, Split::Cols);
    }

    #[test]
    fn deep_merge_overrides_scalars_and_merges_maps() {
        let a = serde_json::json!({ "default": { "worktree": false } });
        let b = serde_json::json!({ "default": { "worktree": true }, "dev": { "worktree": true } });
        let m = deep_merge(a, b);
        assert_eq!(
            m,
            serde_json::json!({
                "default": { "worktree": true },
                "dev": { "worktree": true }
            })
        );
    }

    #[test]
    fn deep_merge_arrays_overwrite() {
        let a = serde_json::json!({
            "default": { "layout": [{"name": "old", "content": []}] }
        });
        let b = serde_json::json!({
            "default": { "layout": [{"name": "new", "content": []}] }
        });
        let m = deep_merge(a, b);
        assert_eq!(m["default"]["layout"][0]["name"], "new");
        assert_eq!(m["default"]["layout"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn load_for_walks_up_and_lower_wins() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");
        std::fs::create_dir_all(&child).unwrap();

        std::fs::write(
            parent.join(".twrk.yaml"),
            "default:\n  worktree: false\n  layout:\n    - content:\n        - command: from-parent\n",
        )
        .unwrap();
        std::fs::write(
            child.join(".twrk.yaml"),
            "default:\n  worktree: true\n  layout:\n    - content:\n        - command: from-child\n",
        )
        .unwrap();

        let cfg = load_for(&child).unwrap();
        let group = &cfg["default"];
        assert_eq!(group.worktree, Some(true));
        let layout = group.layout.as_ref().unwrap();
        let Element::Pane(pane) = &layout[0].content[0];
        assert_eq!(pane.command, "from-child");
    }

    #[test]
    fn load_for_prefers_toml_over_yaml_in_same_dir() {
        let root = tempfile::tempdir().unwrap();
        // toml has only an empty "default" group with no `worktree`
        std::fs::write(root.path().join(".twrk.toml"), "[default]\n").unwrap();
        // yaml would set worktree, but yaml should never be read
        std::fs::write(
            root.path().join(".twrk.yaml"),
            "default:\n  worktree: true\n",
        )
        .unwrap();
        let cfg = load_for(root.path()).unwrap();
        assert_eq!(cfg["default"].worktree, None);
    }

    #[test]
    fn load_for_returns_default_when_no_files() {
        let root = tempfile::tempdir().unwrap();
        let cfg = load_for(root.path()).unwrap();
        assert!(cfg.is_empty());
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

    #[test]
    fn null_content_deserialises_to_empty_vec() {
        let yaml = "default:\n  layout:\n    - name: a\n    - name: b\n      content:\n";
        let cfg: Config = serde_yml::from_str(yaml).unwrap();
        let layout = cfg["default"].layout.as_ref().unwrap();
        assert_eq!(layout.len(), 2);
        assert!(layout[0].content.is_empty());
        assert!(layout[1].content.is_empty());
    }

    #[test]
    fn group_worktree_returns_value_or_none() {
        let cfg = expected();
        assert_eq!(group_worktree(&cfg, "default"), Some(false));
        assert_eq!(group_worktree(&cfg, "dev"), Some(true));
        assert_eq!(group_worktree(&cfg, "missing"), None);
    }
}
