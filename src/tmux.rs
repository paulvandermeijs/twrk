#![allow(dead_code)]

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::config::{Element, Layout, Split, Window};

#[must_use]
pub fn build_commands(session: &str, cwd: &Path, layout: &Layout) -> Vec<Vec<String>> {
    let mut cmds: Vec<Vec<String>> = Vec::new();
    let cwd_s = cwd.display().to_string();
    let layout = ensure_named_windows(layout);

    for (idx, window) in layout.iter().enumerate() {
        let win_name = window.name.as_deref().unwrap();
        if idx == 0 {
            cmds.push(vec![
                "new-session".into(),
                "-d".into(),
                "-s".into(),
                session.into(),
                "-n".into(),
                win_name.into(),
                "-c".into(),
                cwd_s.clone(),
            ]);
        } else {
            cmds.push(vec![
                "new-window".into(),
                "-t".into(),
                format!("{session}:"),
                "-n".into(),
                win_name.into(),
                "-c".into(),
                cwd_s.clone(),
            ]);
        }

        for (pane_idx, element) in window.content.iter().enumerate() {
            let Element::Pane(pane) = element;
            if pane_idx > 0 {
                cmds.push(vec![
                    "split-window".into(),
                    split_flag(window.split).into(),
                    "-t".into(),
                    format!("{session}:{win_name}"),
                    "-c".into(),
                    cwd_s.clone(),
                ]);
            }
            if let Some(name) = &pane.name {
                cmds.push(vec![
                    "select-pane".into(),
                    "-t".into(),
                    format!("{session}:{win_name}"),
                    "-T".into(),
                    name.clone(),
                ]);
            }
            cmds.push(vec![
                "send-keys".into(),
                "-t".into(),
                format!("{session}:{win_name}"),
                pane.command.clone(),
                "Enter".into(),
            ]);
        }

        cmds.push(vec![
            "select-pane".into(),
            "-t".into(),
            format!("{session}:{win_name}.{{top-left}}"),
        ]);
    }

    if let Some(first) = layout.first() {
        cmds.push(vec![
            "select-window".into(),
            "-t".into(),
            format!("{session}:{}", first.name.as_deref().unwrap()),
        ]);
    }

    cmds
}

#[must_use]
pub fn session_exists(session: &str) -> bool {
    Command::new("tmux")
        .args(["has-session", "-t", &format!("={session}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn run(commands: &[Vec<String>]) -> Result<()> {
    for cmd in commands {
        let status = Command::new("tmux")
            .args(cmd)
            .status()
            .with_context(|| format!("failed to spawn tmux {cmd:?}"))?;
        if !status.success() {
            bail!(
                "tmux {} failed (exit {status})",
                cmd.first().map_or("", String::as_str)
            );
        }
    }
    Ok(())
}

pub fn attach_or_switch(session: &str) -> Result<()> {
    let inside = std::env::var("TMUX").is_ok();
    let args: &[&str] = if inside {
        &["switch-client", "-t"]
    } else {
        &["attach", "-t"]
    };
    let status = Command::new("tmux")
        .args(args)
        .arg(session)
        .status()
        .context("failed to spawn tmux attach/switch-client")?;
    if !status.success() {
        bail!("tmux failed to attach/switch (exit {status})");
    }
    Ok(())
}

fn split_flag(split: Split) -> &'static str {
    match split {
        Split::Cols => "-h",
        Split::Rows => "-v",
    }
}

fn ensure_named_windows(layout: &Layout) -> Vec<Window> {
    layout
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let mut w = w.clone();
            if w.name.is_none() {
                w.name = Some(format!("win{}", i + 1));
            }
            w
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Pane;
    use std::path::PathBuf;

    fn pane(name: Option<&str>, cmd: &str) -> Element {
        Element::Pane(Pane {
            name: name.map(str::to_string),
            command: cmd.to_string(),
        })
    }

    fn layout_two_windows() -> Layout {
        vec![
            Window {
                name: Some("Dev".into()),
                split: Split::Cols,
                content: vec![pane(Some("Claude"), "claude"), pane(Some("Editor"), "hx")],
            },
            Window {
                name: Some("Logs".into()),
                split: Split::Rows,
                content: vec![pane(None, "tail -f a"), pane(None, "tail -f b")],
            },
        ]
    }

    #[test]
    fn first_window_uses_new_session() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows());
        assert_eq!(cmds[0][0], "new-session");
        assert!(cmds[0].contains(&"-s".into()));
        assert!(cmds[0].contains(&"s".into()));
        assert!(cmds[0].contains(&"Dev".into()));
        assert!(cmds[0].contains(&"/tmp".into()));
    }

    #[test]
    fn second_pane_in_first_window_uses_horizontal_split() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows());
        let split = cmds.iter().find(|c| c[0] == "split-window").unwrap();
        assert!(split.contains(&"-h".into()));
        assert!(split.contains(&"s:Dev".into()));
    }

    #[test]
    fn rows_window_uses_vertical_split() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows());
        let splits: Vec<_> = cmds.iter().filter(|c| c[0] == "split-window").collect();
        assert_eq!(splits.len(), 2);
        assert!(splits[1].contains(&"-v".into()));
        assert!(splits[1].contains(&"s:Logs".into()));
    }

    #[test]
    fn second_window_uses_new_window() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows());
        let nw = cmds.iter().find(|c| c[0] == "new-window").unwrap();
        assert!(nw.contains(&"Logs".into()));
        assert!(nw.contains(&"s:".into()));
    }

    #[test]
    fn unnamed_window_gets_default_name() {
        let layout = vec![Window {
            name: None,
            split: Split::Cols,
            content: vec![pane(None, "ls")],
        }];
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout);
        assert!(cmds[0].contains(&"win1".into()));
    }

    #[test]
    fn final_select_window_targets_first() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows());
        let last = cmds.last().unwrap();
        assert_eq!(last[0], "select-window");
        assert!(last.contains(&"s:Dev".into()));
    }

    #[test]
    fn pane_title_set_when_named() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows());
        let titles: Vec<_> = cmds
            .iter()
            .filter(|c| c[0] == "select-pane" && c.contains(&"-T".into()))
            .collect();
        assert!(titles.iter().any(|c| c.contains(&"Claude".into())));
        assert!(titles.iter().any(|c| c.contains(&"Editor".into())));
    }
}
