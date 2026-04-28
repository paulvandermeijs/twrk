mod cli;
mod config;
mod git;
mod path;
mod picker;
mod run;
mod session;
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
        let roots = workspace::roots()?;
        let projects = workspace::list_projects(&roots);
        picker::pick(&projects)?
    };

    let cfg = config::load_for(&project_dir)?;
    let want_worktree = args.worktree.or(cfg.worktree).unwrap_or(false);
    let name = args.name.clone().unwrap_or_else(session::random_name);

    let session_cwd: PathBuf = if want_worktree {
        match git::repo_root(&project_dir) {
            Some(root) => git::ensure_worktree(&root, &name)?,
            None => project_dir.clone(),
        }
    } else {
        project_dir.clone()
    };

    if let Some(cmd) = args.command.as_deref() {
        return run::run_in(&session_cwd, cmd);
    }

    let folder_name = project_dir
        .file_name()
        .and_then(|s| s.to_str())
        .context("project path has no folder name")?;
    let session_name = session::compose(folder_name, &name);

    if !tmux::session_exists(&session_name) {
        let layout = config::resolve_layout(&cfg, &args.layout);
        let cmds = tmux::build_commands(&session_name, &session_cwd, &layout);
        tmux::run(&cmds)?;
    }
    tmux::attach_or_switch(&session_name)
}
