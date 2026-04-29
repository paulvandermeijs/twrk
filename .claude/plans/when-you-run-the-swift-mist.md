# Fix nested-worktree creation when `twrk` is run inside a worktree

## Context

When `twrk` is invoked from inside an existing linked worktree (e.g. `/Users/paulvandermeijs/Workspace/twrk/.worktrees/lively-goat`), it currently creates the new worktree *inside* the existing one (e.g. `.worktrees/lively-goat/.worktrees/quiet-cat`) and names the tmux session `lively-goat-quiet-cat`. The intended behavior is that worktrees always sit alongside one another under the **main repository's** `.worktrees/` directory and the session is named after the main project (e.g. `twrk-quiet-cat`).

Root cause: `git::repo_root()` uses `git rev-parse --show-toplevel`, which returns the current working tree's root — that is the worktree itself when invoked from inside one. The result is then used both as the parent for `ensure_worktree` and (indirectly via `project_dir.file_name()`) for the session name.

## Approach

Make `git::repo_root()` resolve to the **main worktree** regardless of which worktree the caller is in, then use that resolved root as the source of the session's "folder name" whenever we successfully resolved a repo root for worktree creation.

This is the minimum change that fixes both symptoms with one well-defined Git invocation.

## Changes

### 1. `src/git.rs` — resolve the main worktree

Replace the body of `repo_root()` (lines 6–20) so it asks Git for the main `.git` directory and returns its parent:

```rust
#[must_use]
pub fn repo_root(path: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let common_dir = PathBuf::from(s.trim());
    common_dir.parent().map(Path::to_path_buf)
}
```

Why `--git-common-dir` (not `--show-toplevel` or `--git-dir`):
- In the main worktree, `--git-dir` and `--git-common-dir` both point at `<repo>/.git`.
- In a linked worktree, `--git-dir` points at `<repo>/.git/worktrees/<name>` while `--git-common-dir` still points at `<repo>/.git`.
- The parent of `--git-common-dir` is therefore the main repo's working tree in both cases.
- `--path-format=absolute` (Git ≥ 2.31) normalizes the result so `.parent()` is meaningful regardless of the current directory.

### 2. `src/main.rs` — derive `folder_name` from the resolved repo root

Currently (`main.rs` lines 49–69) `folder_name` is always taken from `project_dir.file_name()`. When a worktree is being created we want the *main repo's* directory name instead. Restructure the worktree branch to also surface the directory whose name should drive the session:

```rust
let (session_cwd, worktree_name, folder_source): (PathBuf, Option<String>, PathBuf) =
    if want_worktree {
        match git::repo_root(&project_dir) {
            Some(root) => {
                let name = name_override.unwrap_or_else(session::random_name);
                let path = git::ensure_worktree(&root, &name)?;
                (path, Some(name), root)
            }
            None => (project_dir.clone(), None, project_dir.clone()),
        }
    } else {
        (project_dir.clone(), None, project_dir.clone())
    };

let folder_name = folder_source
    .file_name()
    .and_then(|s| s.to_str())
    .context("project path has no folder name")?;
```

Net effect:
- Non-worktree path: unchanged (`folder_source == project_dir`).
- Worktree path with no Git repo: unchanged (falls back to `project_dir`).
- Worktree path inside a real repo: `folder_source` is now the main worktree, so both the new worktree location and the session name use the project's real directory name.

### 3. `src/git.rs` — add a regression test

Add a test that exercises the bug scenario explicitly:

```rust
#[test]
fn repo_root_returns_main_repo_from_inside_a_worktree() {
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    let wt = ensure_worktree(dir.path(), "feat-y").unwrap();
    let root = repo_root(&wt).expect("repo_root should resolve from inside a worktree");
    assert_eq!(
        root.canonicalize().unwrap(),
        dir.path().canonicalize().unwrap()
    );
}
```

The existing tests (`repo_root_returns_none_for_non_repo`, `repo_root_returns_root_for_subdir`, `ensure_worktree_creates_and_is_idempotent`) continue to pass because `--git-common-dir` returns `.git` in a normal repo and fails outside one.

## Critical files

- `src/git.rs` — lines 6–20 (rewrite `repo_root`); add a new test in the `tests` module.
- `src/main.rs` — lines 49–69 (introduce `folder_source` and use it for `folder_name`).

No config-schema or CLI changes are needed.

## Verification

1. `cargo test` — confirms `repo_root_returns_main_repo_from_inside_a_worktree` passes and no existing test regresses.
2. `cargo build --release`.
3. Manual end-to-end check from this very worktree:
   - `cd /Users/paulvandermeijs/Workspace/twrk/.worktrees/lively-goat`
   - Run the rebuilt `twrk -w` (e.g. via `cargo run -- -w`).
   - Confirm:
     - the new worktree appears at `/Users/paulvandermeijs/Workspace/twrk/.worktrees/<random-name>` (a sibling, not a child),
     - the tmux session is named `twrk-<random-name>` (no `lively-goat-` prefix),
     - and that running the same command from the main repo (`/Users/paulvandermeijs/Workspace/twrk`) still behaves as before.
4. `git worktree list` should show the new worktree as a sibling of `lively-goat`, attached to the main repo.
