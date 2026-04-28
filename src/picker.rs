use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use inquire::Select;

const PAGE_SIZE: usize = 10;

pub fn pick(projects: &[PathBuf]) -> Result<PathBuf> {
    if projects.is_empty() {
        bail!("no projects found in workspace roots");
    }
    let labels: Vec<String> = projects
        .iter()
        .map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string()
        })
        .collect();
    let chosen = Select::new("Select a project", labels.clone())
        .with_page_size(PAGE_SIZE)
        .prompt()
        .context("project selection cancelled")?;
    let idx = labels
        .iter()
        .position(|l| l == &chosen)
        .context("selected project not found in list")?;
    Ok(projects[idx].clone())
}
