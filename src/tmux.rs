use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::config::{Element, Layout, Split, Window};

#[must_use]
pub fn build_commands(
    session: &str,
    cwd: &Path,
    layout: &Layout,
    env: &[(&str, &str)],
) -> Vec<Vec<String>> {
    let mut cmds: Vec<Vec<String>> = Vec::new();
    let cwd_s = cwd.display().to_string();
    let layout = ensure_named_windows(layout);

    for (idx, window) in layout.iter().enumerate() {
        let win_name = window.name.as_deref().unwrap();
        let first_pane = window.content.first().map(|e| {
            let Element::Pane(p) = e;
            p
        });
        let first_cmd = first_pane
            .map(|p| p.command.as_str())
            .filter(|c| !c.is_empty());

        let mut create = if idx == 0 {
            vec![
                "new-session".into(),
                "-d".into(),
                "-s".into(),
                session.into(),
                "-n".into(),
                win_name.into(),
                "-c".into(),
                cwd_s.clone(),
            ]
        } else {
            vec![
                "new-window".into(),
                "-t".into(),
                format!("{session}:"),
                "-n".into(),
                win_name.into(),
                "-c".into(),
                cwd_s.clone(),
            ]
        };
        push_env(&mut create, env);
        if let Some(cmd) = first_cmd {
            create.push(cmd.into());
        }
        cmds.push(create);

        if let Some(pane) = first_pane
            && let Some(name) = &pane.name
        {
            cmds.push(vec![
                "select-pane".into(),
                "-t".into(),
                format!("{session}:{win_name}"),
                "-T".into(),
                name.clone(),
            ]);
        }

        for element in window.content.iter().skip(1) {
            let Element::Pane(pane) = element;
            let mut split = vec![
                "split-window".into(),
                split_flag(window.split).into(),
                "-t".into(),
                format!("{session}:{win_name}"),
                "-c".into(),
                cwd_s.clone(),
            ];
            push_env(&mut split, env);
            if !pane.command.is_empty() {
                split.push(pane.command.clone());
            }
            cmds.push(split);
            if let Some(name) = &pane.name {
                cmds.push(vec![
                    "select-pane".into(),
                    "-t".into(),
                    format!("{session}:{win_name}"),
                    "-T".into(),
                    name.clone(),
                ]);
            }
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
        let output = Command::new("tmux")
            .args(cmd)
            .output()
            .with_context(|| format!("failed to spawn tmux {cmd:?}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "tmux {} failed (exit {}): {}",
                cmd.first().map_or("", String::as_str),
                output.status,
                stderr.trim()
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

fn push_env(args: &mut Vec<String>, env: &[(&str, &str)]) {
    for (key, value) in env {
        args.push("-e".into());
        args.push(format!("{key}={value}"));
    }
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
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
        assert_eq!(cmds[0][0], "new-session");
        assert!(cmds[0].contains(&"-s".into()));
        assert!(cmds[0].contains(&"s".into()));
        assert!(cmds[0].contains(&"Dev".into()));
        assert!(cmds[0].contains(&"/tmp".into()));
    }

    #[test]
    fn second_pane_in_first_window_uses_horizontal_split() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
        let split = cmds.iter().find(|c| c[0] == "split-window").unwrap();
        assert!(split.contains(&"-h".into()));
        assert!(split.contains(&"s:Dev".into()));
    }

    #[test]
    fn rows_window_uses_vertical_split() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
        let splits: Vec<_> = cmds.iter().filter(|c| c[0] == "split-window").collect();
        assert_eq!(splits.len(), 2);
        assert!(splits[1].contains(&"-v".into()));
        assert!(splits[1].contains(&"s:Logs".into()));
    }

    #[test]
    fn second_window_uses_new_window() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
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
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout, &[]);
        assert!(cmds[0].contains(&"win1".into()));
    }

    #[test]
    fn final_select_window_targets_first() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
        let last = cmds.last().unwrap();
        assert_eq!(last[0], "select-window");
        assert!(last.contains(&"s:Dev".into()));
    }

    #[test]
    fn pane_title_set_when_named() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
        let titles: Vec<_> = cmds
            .iter()
            .filter(|c| c[0] == "select-pane" && c.contains(&"-T".into()))
            .collect();
        assert!(titles.iter().any(|c| c.contains(&"Claude".into())));
        assert!(titles.iter().any(|c| c.contains(&"Editor".into())));
    }

    #[test]
    fn env_vars_are_added_to_new_session() {
        let env = [("TWRK_CONFIG", "dev"), ("TWRK_WORKTREE", "1")];
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
        let ns = &cmds[0];
        assert_eq!(ns[0], "new-session");
        let pairs: Vec<&String> = ns
            .windows(2)
            .filter(|w| w[0] == "-e")
            .map(|w| &w[1])
            .collect();
        assert!(pairs.iter().any(|p| p.as_str() == "TWRK_CONFIG=dev"));
        assert!(pairs.iter().any(|p| p.as_str() == "TWRK_WORKTREE=1"));
    }

    #[test]
    fn env_vars_are_added_to_new_window() {
        let env = [("TWRK_CONFIG", "dev")];
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
        let nw = cmds.iter().find(|c| c[0] == "new-window").unwrap();
        let pairs: Vec<&String> = nw
            .windows(2)
            .filter(|w| w[0] == "-e")
            .map(|w| &w[1])
            .collect();
        assert!(pairs.iter().any(|p| p.as_str() == "TWRK_CONFIG=dev"));
    }

    #[test]
    fn env_vars_are_added_to_split_window() {
        let env = [("TWRK_CONFIG", "dev")];
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
        let sw = cmds.iter().find(|c| c[0] == "split-window").unwrap();
        let pairs: Vec<&String> = sw
            .windows(2)
            .filter(|w| w[0] == "-e")
            .map(|w| &w[1])
            .collect();
        assert!(pairs.iter().any(|p| p.as_str() == "TWRK_CONFIG=dev"));
    }

    #[test]
    fn env_var_precedes_trailing_shell_command() {
        let env = [("TWRK_CONFIG", "dev")];
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
        let ns = &cmds[0];
        let last = ns.last().unwrap();
        assert_eq!(last, "claude");
        let last_e = ns.iter().rposition(|s| s == "-e").unwrap();
        assert!(last_e < ns.len() - 2);
    }

    #[test]
    fn no_env_means_no_dash_e_flags() {
        let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
        for cmd in &cmds {
            assert!(
                !cmd.iter().any(|s| s == "-e"),
                "unexpected -e in {cmd:?}"
            );
        }
    }
}
