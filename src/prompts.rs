use anyhow::{Context, Result};

pub fn pick_config(group_names: &[String]) -> Result<String> {
    let items: Vec<(String, String, &'static str)> = group_names
        .iter()
        .map(|name| (name.clone(), name.clone(), ""))
        .collect();
    let selected = cliclack::select("Select a config")
        .items(&items)
        .interact()
        .context("config selection cancelled")?;
    Ok(selected)
}

pub fn pick_worktree(default: bool, placeholder_name: &str) -> Result<Option<String>> {
    let create = cliclack::confirm("Create a worktree?")
        .initial_value(default)
        .interact()
        .context("worktree confirmation cancelled")?;
    if !create {
        return Ok(None);
    }
    let typed = cliclack::input("Worktree name")
        .placeholder(placeholder_name)
        .required(false)
        .interact::<String>()
        .context("worktree name input cancelled")?;
    if typed.trim().is_empty() {
        Ok(Some(placeholder_name.to_string()))
    } else {
        Ok(Some(typed))
    }
}

#[must_use]
pub fn resolve_group_name(flag: Option<&str>, picked: Option<&str>) -> String {
    if let Some(value) = flag {
        return value.to_string();
    }
    if let Some(value) = picked {
        return value.to_string();
    }
    "default".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_set_returns_flag() {
        assert_eq!(
            resolve_group_name(Some("from-flag"), Some("from-picker")),
            "from-flag"
        );
        assert_eq!(resolve_group_name(Some("from-flag"), None), "from-flag");
    }

    #[test]
    fn flag_unset_picked_some_returns_picked() {
        assert_eq!(
            resolve_group_name(None, Some("from-picker")),
            "from-picker"
        );
    }

    #[test]
    fn both_unset_returns_default() {
        assert_eq!(resolve_group_name(None, None), "default");
    }
}
