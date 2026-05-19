# Worktree setup hooks — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-group `setup` field to the twrkfile so a list of shell commands runs automatically right after a new worktree is created (and before the tmux session starts). Mirrors Superset's `setup` config from [superset-sh/superset](https://github.com/superset-sh/superset).

**Architecture:** Add an optional `setup: Vec<String>` to the `Group` struct in `src/config.rs`. Modify `git::ensure_worktree` to return `(PathBuf, bool)` so callers know whether the worktree was *newly created* (idempotent re-use must not re-run setup). Introduce a new `src/setup.rs` module whose `run` function executes each command via `sh -c`, inheriting stdio and exporting the same `TWRK_*` env vars used for tmux plus `TWRK_WORKTREE_NAME` and `TWRK_REPO_ROOT`. Wire it into `main.rs` between worktree creation and tmux launch — if setup fails, twrk bails before any tmux session is spawned.

**Tech Stack:** Rust 2024, anyhow, clap, serde, tmux CLI, `sh -c` for command execution.

**Design decisions:**
- Per-group, not top-level — matches the existing `Group` shape (`worktree` + `layout`).
- `setup` is an array of shell command strings (matching Superset's syntax). Each is run via `sh -c "<command>"` sequentially. First non-zero exit aborts the run.
- Setup runs only when `ensure_worktree` reports `created == true`. Re-attaching to an existing worktree (e.g. `twrk -w my-feature` twice) does not re-run setup — same idempotency contract as the worktree itself.
- Setup runs in the **new worktree** as cwd. So `cp ../.env .env` style commands work (parent dir is `.worktrees/`, and the source repo root is exposed as `TWRK_REPO_ROOT` for absolute references).
- Setup inherits twrk's stdio so the user sees output live (no buffering, no spinner) — this matches what someone bootstrapping a project expects to see.
- Setup runs **only when a worktree is created**, not on the non-worktree path. Project-level bootstrap is a distinct feature and out of scope.
- Env vars exposed to setup commands: `TWRK_CONFIG`, `TWRK_WORKTREE=1`, `TWRK_WORKTREE_NAME=<name>`, `TWRK_REPO_ROOT=<absolute path>`. Naming aligns with the existing tmux env vars (`TWRK_CONFIG`, `TWRK_WORKTREE`) so users see one consistent namespace.
- Teardown is **out of scope** — see the "Out of scope" section. tl;dr: twrk has no worktree-removal command yet, and adding one is an independent feature.

---

## File Structure

- `src/config.rs` — `Group` gains `setup: Option<Vec<String>>` (with `skip_serializing_if`). Add a `group_setup` helper alongside `group_worktree`. Tests cover parsing across YAML/TOML/JSON and the helper.
- `src/git.rs` — `ensure_worktree` returns `(PathBuf, bool)` instead of `PathBuf`. The `bool` is `true` when this invocation created the worktree, `false` when the path already existed. Existing tests updated; a new test confirms the second call reports `false`.
- `src/setup.rs` — **new module**. `pub fn run(workdir: &Path, env: &[(&str, &str)], commands: &[String]) -> Result<()>`. Spawns each command via `sh -c`, inheriting stdio and `env`, with `workdir` as cwd. Returns `Err` on the first non-zero exit. Integration tests use real `sh` against a `tempfile::tempdir()` workdir.
- `src/main.rs` — declares the new `setup` module and, in `real_main`, after `ensure_worktree` succeeds with `created == true`, builds the setup env vec and calls `setup::run`. The pre-existing `env` for tmux gains its `TWRK_WORKTREE_NAME` member at this seam too — they can share a builder, but for simplicity each call site assembles what it needs.
- `README.md` — new "Worktree setup" subsection after "Project config", with an example using `setup`.

No new dependencies. No new files beyond `src/setup.rs`.

---

## Task 1: Add `setup` to the config schema (TDD)

**Files:**
- Modify: `src/config.rs:9-15` (the `Group` struct)
- Modify: `src/config.rs:96-99` (add `group_setup` next to `group_worktree`)
- Test: `src/config.rs` (inline `#[cfg(test)] mod tests` — add new parsing + helper tests)

- [ ] **Step 1: Write a failing test for `setup` parsing from YAML**

In `src/config.rs`, in the `tests` module, append:

```rust
#[test]
fn parses_setup_from_yaml() {
    let yaml = r#"
default:
  setup:
    - cp ../.env .env
    - bun install
"#;
    let cfg: Config = serde_yml::from_str(yaml).unwrap();
    let group = &cfg["default"];
    let expected = vec!["cp ../.env .env".to_string(), "bun install".to_string()];
    assert_eq!(group.setup.as_deref(), Some(expected.as_slice()));
}

#[test]
fn parses_setup_missing_as_none() {
    let yaml = "default:\n  worktree: true\n";
    let cfg: Config = serde_yml::from_str(yaml).unwrap();
    assert!(cfg["default"].setup.is_none());
}

#[test]
fn parses_setup_empty_array() {
    let yaml = "default:\n  setup: []\n";
    let cfg: Config = serde_yml::from_str(yaml).unwrap();
    let empty: Vec<String> = Vec::new();
    assert_eq!(cfg["default"].setup.as_deref(), Some(empty.as_slice()));
}

#[test]
fn group_setup_returns_value_or_none() {
    let mut cfg = Config::new();
    cfg.insert(
        "with".into(),
        Group {
            worktree: None,
            setup: Some(vec!["echo hi".into()]),
            layout: None,
        },
    );
    cfg.insert(
        "without".into(),
        Group {
            worktree: None,
            setup: None,
            layout: None,
        },
    );
    let expected = vec!["echo hi".to_string()];
    assert_eq!(group_setup(&cfg, "with"), Some(expected.as_slice()));
    assert_eq!(group_setup(&cfg, "without"), None);
    assert_eq!(group_setup(&cfg, "missing"), None);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test --lib config:: 2>&1 | tail -20
```

Expected: compile errors — `Group` has no field `setup`, `group_setup` is not defined.

- [ ] **Step 3: Add the `setup` field to `Group`**

In `src/config.rs`, replace the current `Group` struct (lines 9-15) with:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Group {
    pub worktree: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
}
```

- [ ] **Step 4: Add the `group_setup` helper**

In `src/config.rs`, immediately after the `group_worktree` function (currently ending around line 99), append:

```rust
#[must_use]
pub fn group_setup<'a>(cfg: &'a Config, id: &str) -> Option<&'a [String]> {
    cfg.get(id).and_then(|g| g.setup.as_deref())
}
```

- [ ] **Step 5: Update the pre-existing `expected()` test fixture so it constructs `Group` with the new field**

The existing `expected()` helper builds `Group { worktree: ..., layout: ... }`. Since the struct now has a third field, those constructions need `setup: None` to compile. Patch both `Group` literals in `expected()` to:

```rust
Group {
    worktree: Some(false), // or Some(true) for dev
    setup: None,
    layout: Some(...),
}
```

(Two call sites — the `default` group and the `dev` group inside `expected()`.)

- [ ] **Step 6: Run all config tests to verify they pass**

Run:

```bash
cargo test --lib config:: 2>&1 | tail -20
```

Expected: all config tests pass — including the four new ones.

- [ ] **Step 7: Commit**

```bash
git add src/config.rs
git commit -m "feat: Add per-group setup commands to twrk config

- Add \`Group.setup: Option<Vec<String>>\` to the twrkfile schema
- Add \`group_setup\` helper alongside \`group_worktree\`
- Cover parsing (YAML), missing-field default, and helper behaviour with unit tests"
```

---

## Task 2: Have `ensure_worktree` report whether the worktree was newly created (TDD)

**Files:**
- Modify: `src/git.rs:23-46` (the `ensure_worktree` function body and signature)
- Modify: `src/git.rs:84-123` (existing tests that call `ensure_worktree`)

- [ ] **Step 1: Write a failing test that distinguishes first vs. second call**

In `src/git.rs`, in the `tests` module, replace the existing `ensure_worktree_creates_and_is_idempotent` test with:

```rust
#[test]
fn ensure_worktree_reports_created_then_not_created() {
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    let (p1, created1) = ensure_worktree(dir.path(), "feat-x").unwrap();
    assert!(p1.is_dir());
    assert!(created1, "first call should report created=true");
    let (p2, created2) = ensure_worktree(dir.path(), "feat-x").unwrap();
    assert_eq!(p1, p2);
    assert!(!created2, "second call should report created=false");
}
```

Also patch the other test that uses `ensure_worktree` — `repo_root_returns_main_repo_from_inside_a_worktree` (currently `let wt = ensure_worktree(...).unwrap();`). Replace with:

```rust
let (wt, _) = ensure_worktree(dir.path(), "feat-y").unwrap();
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test --lib git:: 2>&1 | tail -20
```

Expected: compile errors — destructuring `(PathBuf, bool)` from a function that still returns `PathBuf`.

- [ ] **Step 3: Update `ensure_worktree` to return `(PathBuf, bool)`**

In `src/git.rs`, replace the current `ensure_worktree` (lines 23-46) with:

```rust
pub fn ensure_worktree(repo_root: &Path, name: &str) -> Result<(PathBuf, bool)> {
    let target = repo_root.join(".worktrees").join(name);
    if target.is_dir() {
        return Ok((target, false));
    }
    std::fs::create_dir_all(target.parent().unwrap())
        .with_context(|| format!("could not create {}", target.parent().unwrap().display()))?;
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["worktree", "add", "-b", name])
        .arg(&target)
        .output()
        .context("failed to spawn `git worktree add`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git worktree add failed (exit status {}): {}",
            output.status,
            stderr.trim()
        );
    }
    Ok((target, true))
}
```

- [ ] **Step 4: Run git tests to verify they pass**

Run:

```bash
cargo test --lib git:: 2>&1 | tail -20
```

Expected: all git tests pass.

- [ ] **Step 5: Patch the caller in `main.rs` so the project still builds**

In `src/main.rs`, find the call site at line 96 (`let path = git::ensure_worktree(&root, &name)?;`) and replace with:

```rust
let (path, _created) = git::ensure_worktree(&root, &name)?;
```

(We'll use `_created` for now and wire it up properly in Task 4.)

- [ ] **Step 6: Run the full test suite**

Run:

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass — no other call sites should break.

- [ ] **Step 7: Commit**

```bash
git add src/git.rs src/main.rs
git commit -m "refactor: Report newly-created status from \`ensure_worktree\`

- Change return type from \`PathBuf\` to \`(PathBuf, bool)\`
- \`bool\` is \`true\` on first creation, \`false\` when the path already existed
- Lets callers conditionally run one-shot hooks (setup) only on fresh worktrees
- Tests updated to cover both branches"
```

---

## Task 3: Implement the setup runner (`src/setup.rs`) (TDD)

**Files:**
- Create: `src/setup.rs`
- Test: `src/setup.rs` (inline `#[cfg(test)] mod tests`)

- [ ] **Step 1: Add the module declaration in `main.rs`**

In `src/main.rs`, add `mod setup;` to the module list at the top, in alphabetical order. Final block should look like:

```rust
mod cli;
mod config;
mod git;
mod path;
mod picker;
mod prompts;
mod session;
mod setup;
mod theme;
mod tmux;
mod workspace;
```

- [ ] **Step 2: Create `src/setup.rs` with failing tests**

Write this content to `src/setup.rs`:

```rust
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn run(workdir: &Path, env: &[(&str, &str)], commands: &[String]) -> Result<()> {
    for cmd in commands {
        let mut child = Command::new("sh");
        child
            .arg("-c")
            .arg(cmd)
            .current_dir(workdir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        for (key, value) in env {
            child.env(key, value);
        }
        let status = child
            .status()
            .with_context(|| format!("failed to spawn setup command: {cmd}"))?;
        if !status.success() {
            bail!("setup command failed (exit {}): {cmd}", status);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn runs_single_command_in_workdir() {
        let dir = tempdir().unwrap();
        let cmds = vec!["touch marker".to_string()];
        run(dir.path(), &[], &cmds).unwrap();
        assert!(dir.path().join("marker").exists());
    }

    #[test]
    fn runs_multiple_commands_sequentially() {
        let dir = tempdir().unwrap();
        let cmds = vec![
            "echo first > a".to_string(),
            "echo second > b".to_string(),
        ];
        run(dir.path(), &[], &cmds).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("a")).unwrap().trim(), "first");
        assert_eq!(fs::read_to_string(dir.path().join("b")).unwrap().trim(), "second");
    }

    #[test]
    fn bails_on_first_non_zero_exit() {
        let dir = tempdir().unwrap();
        let cmds = vec![
            "touch before".to_string(),
            "exit 7".to_string(),
            "touch after".to_string(),
        ];
        let err = run(dir.path(), &[], &cmds).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("exit 7") || msg.contains("exit code: 7"), "got: {msg}");
        assert!(dir.path().join("before").exists());
        assert!(!dir.path().join("after").exists(), "should not run after failure");
    }

    #[test]
    fn env_vars_are_visible_to_commands() {
        let dir = tempdir().unwrap();
        let env = [("TWRK_REPO_ROOT", "/tmp/fake-root"), ("TWRK_CONFIG", "dev")];
        let cmds = vec!["printf '%s\\n%s' \"$TWRK_REPO_ROOT\" \"$TWRK_CONFIG\" > out".to_string()];
        run(dir.path(), &env, &cmds).unwrap();
        let out = fs::read_to_string(dir.path().join("out")).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["/tmp/fake-root", "dev"]);
    }

    #[test]
    fn empty_commands_is_noop() {
        let dir = tempdir().unwrap();
        run(dir.path(), &[], &[]).unwrap();
    }
}
```

- [ ] **Step 3: Run the new tests to verify they pass**

Run:

```bash
cargo test --lib setup:: 2>&1 | tail -20
```

Expected: all five setup tests pass. (They should pass on the first run because the implementation in Step 2 is included alongside the tests — this is a single self-contained module, so TDD here is really "write the test list and implementation together, then run".)

If they don't pass: investigate `sh` availability and shell quoting; the tests assume a POSIX `/bin/sh` (always present on macOS/Linux dev machines, which is twrk's target).

- [ ] **Step 4: Commit**

```bash
git add src/setup.rs src/main.rs
git commit -m "feat: Add setup runner that executes worktree-bootstrap commands

- New \`src/setup.rs\` module with \`run(workdir, env, commands)\`
- Each command is run via \`sh -c\` with inherited stdio, fixed cwd, and explicit env
- Bails on the first non-zero exit; subsequent commands are skipped
- Tests cover happy path, sequencing, failure, env visibility, and empty-input noop"
```

---

## Task 4: Wire setup into the `main.rs` flow

**Files:**
- Modify: `src/main.rs:93-100` (the worktree construction block)

- [ ] **Step 1: Replace the worktree branch in `real_main`**

In `src/main.rs`, replace the current block (lines 93-100):

```rust
    let (session_cwd, worktree_name, folder_source): (PathBuf, Option<String>, PathBuf) =
        if want_worktree && let Some(root) = git::repo_root(&project_dir) {
            let name = name_override.unwrap_or_else(session::random_name);
            let (path, _created) = git::ensure_worktree(&root, &name)?;
            (path, Some(name), root)
        } else {
            (project_dir.clone(), None, project_dir.clone())
        };
```

with:

```rust
    let (session_cwd, worktree_name, folder_source): (PathBuf, Option<String>, PathBuf) =
        if want_worktree && let Some(root) = git::repo_root(&project_dir) {
            let name = name_override.unwrap_or_else(session::random_name);
            let (path, created) = git::ensure_worktree(&root, &name)?;
            if created
                && let Some(cmds) = config::group_setup(&cfg, &active_group)
                && !cmds.is_empty()
            {
                let root_s = root.display().to_string();
                let setup_env: Vec<(&str, &str)> = vec![
                    ("TWRK_CONFIG", active_group.as_str()),
                    ("TWRK_WORKTREE", "1"),
                    ("TWRK_WORKTREE_NAME", name.as_str()),
                    ("TWRK_REPO_ROOT", root_s.as_str()),
                ];
                setup::run(&path, &setup_env, cmds)?;
            }
            (path, Some(name), root)
        } else {
            (project_dir.clone(), None, project_dir.clone())
        };
```

Note: `cmds` is `&[String]` from `group_setup`, which matches `setup::run`'s third parameter signature.

- [ ] **Step 2: Build to verify the code compiles**

Run:

```bash
cargo build 2>&1 | tail -5
```

Expected: `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` with no errors.

- [ ] **Step 3: Run the full test suite**

Run:

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass (config + git + setup + tmux + the existing ones).

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: Run setup commands after creating a fresh worktree

- When \`ensure_worktree\` reports \`created == true\` and the resolved group
  has non-empty \`setup\`, invoke \`setup::run\` with the new worktree as cwd
- Setup sees \`TWRK_CONFIG\`, \`TWRK_WORKTREE=1\`, \`TWRK_WORKTREE_NAME\`,
  and \`TWRK_REPO_ROOT\` in its env
- Setup failure aborts twrk before any tmux session is created"
```

---

## Task 5: Manual integration check

**Files:**
- No code changes — runtime verification only.

- [ ] **Step 1: Install the local build**

Run:

```bash
cargo install --path . 2>&1 | tail -5
```

Expected: `Installed package \`twrk v1.2.0\``.

- [ ] **Step 2: Add a temporary `.twrk.yaml` with a setup hook to the twrk repo**

From the twrk repo root, write a temporary config file. The setup will write a marker file into the new worktree, so we can confirm it ran:

```bash
cat > /tmp/twrk-setup-test.yaml <<'EOF'
default:
  worktree: true
  setup:
    - echo "ran setup at $(date -u)" > SETUP_MARKER
    - echo "repo root was $TWRK_REPO_ROOT, config was $TWRK_CONFIG" >> SETUP_MARKER
  layout:
    - name: main
      content:
        - command: cat SETUP_MARKER
EOF
cp /tmp/twrk-setup-test.yaml /Users/paulvandermeijs/Workspace/twrk/.twrk.yaml
```

(We use the parent repo, not this worktree, so the test exercises real worktree creation.)

- [ ] **Step 3: Create a worktree and verify the setup ran**

From outside any tmux session:

```bash
twrk /Users/paulvandermeijs/Workspace/twrk -w
```

You should see the setup output streamed to your terminal *before* the tmux session attaches. Once attached, the first pane's `cat SETUP_MARKER` should display two lines:

```
ran setup at <UTC timestamp>
repo root was /Users/paulvandermeijs/Workspace/twrk, config was default
```

If the marker file is missing or the env vars aren't substituted, the wiring is wrong.

- [ ] **Step 4: Verify re-attaching does *not* re-run setup**

Detach from the session (`prefix d`). From a different shell:

```bash
twrk /Users/paulvandermeijs/Workspace/twrk -w <same-worktree-name-from-step-3>
```

Expected: no setup output, just an immediate attach. The marker file's timestamp inside the worktree should be unchanged.

- [ ] **Step 5: Verify setup failure aborts before tmux**

Edit `/Users/paulvandermeijs/Workspace/twrk/.twrk.yaml` and replace the second setup command with `exit 1`:

```yaml
default:
  worktree: true
  setup:
    - echo "this runs"
    - exit 1
  layout:
    - name: main
      content:
        - command: echo should-not-see-this
```

Then:

```bash
twrk /Users/paulvandermeijs/Workspace/twrk -w
```

Expected: `this runs` appears, then twrk errors out with a "setup command failed (exit 1)" message, and **no tmux session is created**. Verify with `tmux ls` (the new session name should be absent).

- [ ] **Step 6: Clean up**

```bash
rm /Users/paulvandermeijs/Workspace/twrk/.twrk.yaml
rm /tmp/twrk-setup-test.yaml
# Remove the test worktrees from main checkout:
cd /Users/paulvandermeijs/Workspace/twrk
git worktree list   # identify the test worktrees from steps 3-5
git worktree remove .worktrees/<name>   # repeat per worktree
# Kill leftover tmux sessions if any:
tmux kill-session -t <name>
```

No commit — this task is verification only.

---

## Task 6: Document worktree setup in README

**Files:**
- Modify: `README.md` (new subsection between "Project config" and "Session env vars")

- [ ] **Step 1: Find the insertion point**

Run:

```bash
grep -n "^## " README.md
```

Identify the line numbers of `## Project config` and `## Session env vars`. The new section goes between them.

- [ ] **Step 2: Add the new subsection**

Insert this block in `README.md` immediately before the `## Session env vars` heading:

````markdown
## Worktree setup

A config group can declare a `setup` list — shell commands that run automatically after twrk creates a fresh worktree, before the tmux session starts. Re-attaching to an existing worktree does **not** re-run setup.

Commands run sequentially via `sh -c` with the new worktree as the working directory. They inherit twrk's stdio so you see output live. The first non-zero exit aborts; no tmux session is created.

```yaml
dev:
  worktree: true
  setup:
    - cp ../.env .env
    - bun install
  layout:
    - name: dev
      content:
        - { command: bun run dev }
```

The setup commands see these env vars:

- `TWRK_CONFIG` — the resolved config group name.
- `TWRK_WORKTREE` — always `1` while setup runs.
- `TWRK_WORKTREE_NAME` — the worktree's name (the segment after `.worktrees/`).
- `TWRK_REPO_ROOT` — absolute path to the source repo, useful when `..` isn't enough.

````

- [ ] **Step 3: Verify the section landed in the right place**

Run:

```bash
grep -n "^## " README.md
```

Expected: `## Worktree setup` appears between `## Project config` and `## Session env vars`.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: Document the worktree setup hook"
```

---

## Out of scope

- **Teardown commands.** Superset has `teardown` because it owns workspace destruction. twrk does not — there is no `twrk rm <worktree>` subcommand and no session-end hook. Adding teardown is a meaningful new feature: either a new CLI subcommand (`twrk rm <name>` that runs teardown, then `git worktree remove`, then optionally `tmux kill-session`), or wiring into tmux session lifecycle (e.g. `set-hook -g session-closed`). Both are independent designs — handle in a follow-up plan.
- **Top-level setup applying to every group.** Per-group only for now. Users can duplicate or factor into a shared shell script if they want it everywhere.
- **`pre_create` hooks** (running before `git worktree add`). Not requested; the use cases Superset covers (env copy, deps install) are all post-create.
- **Setup on the non-worktree path** (running on plain `twrk <path>` without `-w`). Project-level bootstrap is a distinct feature; setup is intentionally scoped to "I just made a new worktree, prepare it".
- **Backgrounding setup** (running it while tmux launches in parallel). KISS — block until setup finishes so any pane that depends on installed deps doesn't race.
- **A "quiet" / non-streaming mode.** Setup output is always streamed inline — users bootstrapping a fresh worktree generally want to see what's happening.
