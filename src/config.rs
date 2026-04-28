#![allow(dead_code)]

use std::collections::BTreeMap;

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

impl Element {
    #[must_use]
    pub fn as_pane(&self) -> &Pane {
        let Self::Pane(p) = self;
        p
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
}
