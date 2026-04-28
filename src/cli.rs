use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Project path (skips selection). Absolute, relative, or `.`.
    pub path: Option<String>,

    /// Create a worktree. Optional value sets the name; random if omitted.
    #[arg(short = 'w', long, num_args = 0..=1, default_missing_value = "")]
    pub worktree: Option<String>,

    /// Config group id from the config file
    #[arg(short = 'c', long, default_value = "default")]
    pub config: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_path_only() {
        let a = Args::try_parse_from(["twrk", "."]).unwrap();
        assert_eq!(a.path.as_deref(), Some("."));
        assert_eq!(a.config, "default");
        assert_eq!(a.worktree, None);
    }

    #[test]
    fn parses_worktree_flag_without_value() {
        let a = Args::try_parse_from(["twrk", "-w"]).unwrap();
        assert_eq!(a.worktree.as_deref(), Some(""));
    }

    #[test]
    fn parses_worktree_flag_with_name() {
        let a = Args::try_parse_from(["twrk", "-w", "feat"]).unwrap();
        assert_eq!(a.worktree.as_deref(), Some("feat"));
    }

    #[test]
    fn parses_full_form() {
        let a = Args::try_parse_from(["twrk", ".", "-c", "dev", "-w", "feat"]).unwrap();
        assert_eq!(a.path.as_deref(), Some("."));
        assert_eq!(a.config, "dev");
        assert_eq!(a.worktree.as_deref(), Some("feat"));
    }
}
