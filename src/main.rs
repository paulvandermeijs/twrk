mod cli;
mod config;
mod git;
mod path;
mod picker;
mod session;
mod theme;
mod tmux;
mod workspace;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("twrk: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn real_main() -> Result<()> {
    let args = cli::Args::parse();

    let project_dir = if let Some(p) = args.path.as_deref() {
        path::resolve(p)?
    } else {
        theme::install();
        let _ = cliclack::intro(console::style(" twrk ").bold().black().on_color256(213));
        let roots = workspace::roots()?;
        let projects = workspace::list_projects(&roots);
        picker::pick(&projects)?
    };

    let cfg = config::load_for(&project_dir)?;
    let (want_worktree, name_override) = match args.worktree.as_deref() {
        Some("") => (true, None),
        Some(name) => (true, Some(name.to_string())),
        None => (
            config::group_worktree(&cfg, &args.config).unwrap_or(false),
            None,
        ),
    };

    let (session_cwd, worktree_name, folder_source): (PathBuf, Option<String>, PathBuf) =
        if want_worktree && let Some(root) = git::repo_root(&project_dir) {
            let name = name_override.unwrap_or_else(session::random_name);
            let path = git::ensure_worktree(&root, &name)?;
            (path, Some(name), root)
        } else {
            (project_dir.clone(), None, project_dir.clone())
        };

    let folder_name = folder_source
        .file_name()
        .and_then(|s| s.to_str())
        .context("project path has no folder name")?;
    let session_name = match worktree_name.as_deref() {
        Some(name) => session::compose(folder_name, name),
        None => folder_name.to_string(),
    };

    if !tmux::session_exists(&session_name) {
        let layout = config::resolve_layout(&cfg, &args.config);
        let cmds = tmux::build_commands(&session_name, &session_cwd, &layout);
        tmux::run(&cmds)?;
    }
    tmux::attach_or_switch(&session_name)
}
