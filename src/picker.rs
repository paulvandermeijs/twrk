use std::path::PathBuf;

use anyhow::{Context, Result, bail};

const MAX_ROWS: usize = 10;

pub fn pick(projects: &[PathBuf]) -> Result<PathBuf> {
    if projects.is_empty() {
        bail!("no projects found in workspace roots");
    }
    let items: Vec<(PathBuf, String, &'static str)> = projects
        .iter()
        .map(|p| {
            let label = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            (p.clone(), label, "")
        })
        .collect();
    let selected = cliclack::select("Select a project")
        .items(&items)
        .filter_mode()
        .max_rows(MAX_ROWS)
        .interact()
        .context("project selection cancelled")?;
    Ok(selected)
}
