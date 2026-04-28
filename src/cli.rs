use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Project path (skips selection). Absolute, relative, or `.`.
    pub path: Option<String>,

    /// Create a worktree (overrides config)
    #[arg(short = 'w', long)]
    pub worktree: Option<bool>,

    /// Session name; appended to folder name. Random if omitted.
    #[arg(short = 'n', long)]
    pub name: Option<String>,

    /// Layout id from the config file
    #[arg(short = 'l', long, default_value = "default")]
    pub layout: String,

    /// Run an arbitrary command in the project folder instead of tmux
    #[arg(short = 'x', long = "execute")]
    pub command: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_path_only() {
        let a = Args::try_parse_from(["twrk", "."]).unwrap();
        assert_eq!(a.path.as_deref(), Some("."));
        assert_eq!(a.layout, "default");
        assert_eq!(a.worktree, None);
    }

    #[test]
    fn parses_worktree_false() {
        let a = Args::try_parse_from(["twrk", "-w", "false"]).unwrap();
        assert_eq!(a.worktree, Some(false));
    }

    #[test]
    fn parses_full_form() {
        let a =
            Args::try_parse_from(["twrk", ".", "-l", "dev", "-n", "feat", "-w", "true"]).unwrap();
        assert_eq!(a.path.as_deref(), Some("."));
        assert_eq!(a.layout, "dev");
        assert_eq!(a.name.as_deref(), Some("feat"));
        assert_eq!(a.worktree, Some(true));
    }

    #[test]
    fn parses_command_override() {
        let a = Args::try_parse_from(["twrk", "-x", "ls -la"]).unwrap();
        assert_eq!(a.command.as_deref(), Some("ls -la"));
    }

    #[test]
    fn parses_command_long_form() {
        let a = Args::try_parse_from(["twrk", "--execute", "pwd"]).unwrap();
        assert_eq!(a.command.as_deref(), Some("pwd"));
    }
}
