mod cli;
mod config;
mod git;
mod path;
mod picker;
mod prompts;
mod session;
mod theme;
mod tmux;
mod workspace;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> ExitCode {
    let args = cli::Args::parse();
    let is_tty = console::user_attended_stderr();
    let in_picker_mode = args.path.is_none();
    if is_tty {
        theme::install();
        let _ = cliclack::intro(console::style(" twrk ").bold().black().on_color256(213));
    }
    match real_main(args, in_picker_mode, is_tty) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if is_prompt_interrupt(&e) => ExitCode::FAILURE,
        Err(e) if is_tty => {
            let _ = cliclack::outro_cancel(format!("{e:#}"));
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("twrk: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn real_main(args: cli::Args, in_picker_mode: bool, is_tty: bool) -> Result<()> {
    let project_dir = if let Some(p) = args.path.as_deref() {
        path::resolve(p)?
    } else {
        let roots = workspace::roots()?;
        let projects = workspace::list_projects(&roots);
        picker::pick(&projects)?
    };

    if !in_picker_mode && is_tty {
        prompts::show_project(&project_dir);
    }

    let cfg = config::load_for(&project_dir)?;

    let picked_group = if in_picker_mode && args.config.is_none() {
        let mut names: Vec<String> = cfg.keys().cloned().collect();
        if !names.iter().any(|n| n == "default") {
            names.insert(0, "default".to_string());
        }
        if names.len() >= 2 {
            Some(prompts::pick_config(&names)?)
        } else {
            None
        }
    } else {
        None
    };
    let active_group =
        prompts::resolve_group_name(args.config.as_deref(), picked_group.as_deref());

    if !in_picker_mode && is_tty {
        prompts::show_config(&active_group);
    }

    let (want_worktree, name_override) = match args.worktree.as_deref() {
        Some("") => (true, None),
        Some(name) => (true, Some(name.to_string())),
        None => {
            let group_default = config::group_worktree(&cfg, &active_group).unwrap_or(false);
            if in_picker_mode && git::repo_root(&project_dir).is_some() {
                let random = session::random_name();
                match prompts::pick_worktree(group_default, &random)? {
                    None => (false, None),
                    Some(name) => (true, Some(name)),
                }
            } else {
                (group_default, None)
            }
        }
    };

    let (session_cwd, worktree_name, folder_source): (PathBuf, Option<String>, PathBuf) =
        if want_worktree && let Some(root) = git::repo_root(&project_dir) {
            let name = name_override.unwrap_or_else(session::random_name);
            let path = git::ensure_worktree(&root, &name)?;
            (path, Some(name), root)
        } else {
            (project_dir.clone(), None, project_dir.clone())
        };

    if !in_picker_mode && is_tty {
        prompts::show_worktree(worktree_name.as_deref());
    }

    let folder_name = folder_source
        .file_name()
        .and_then(|s| s.to_str())
        .context("project path has no folder name")?;
    let session_name = match worktree_name.as_deref() {
        Some(name) => session::compose(folder_name, name),
        None => folder_name.to_string(),
    };

    if is_tty {
        let _ = cliclack::outro(format!("Launching {session_name}..."));
    }

    if !tmux::session_exists(&session_name) {
        let layout = config::resolve_layout(&cfg, &active_group);
        let cmds = tmux::build_commands(&session_name, &session_cwd, &layout);
        tmux::run(&cmds)?;
    }
    tmux::attach_or_switch(&session_name)
}

fn is_prompt_interrupt(e: &anyhow::Error) -> bool {
    e.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|io_err| io_err.kind() == io::ErrorKind::Interrupted)
    })
}
