# tmux env var injection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Inject `TWRK_CONFIG` and `TWRK_WORKTREE` env vars into every tmux session twrk creates, so commands run from within the session (e.g. `twrk . --worktree --config="$TWRK_CONFIG"`) can re-use the same config.

**Architecture:** `tmux::build_commands` already builds `new-session` / `new-window` / `split-window` invocations. Tmux's `-e KEY=VALUE` flag (supported on all three) sets per-session/window/pane env. We thread a slice of `(&str, &str)` pairs from `main.rs` into `build_commands` and emit one `-e` pair per env var on every create command. Values are computed in `main.rs` from the already-resolved `active_group` and `worktree_name`.

**Tech Stack:** Rust 2024, anyhow, clap, tmux CLI.

**Design decisions:**
- `TWRK_CONFIG` is always set, even when the user didn't pass `-c` — value is the resolved group name (e.g. `default`, `dev`).
- `TWRK_WORKTREE` is set to `1` only when the session opened in a worktree; otherwise unset. (User spec wrote `TWRK_WORKTREE=1` — kept literally; using the worktree name would carry more info but deviates from the spec.)
- `-e` flags are added to **all** create commands (`new-session`, `new-window`, `split-window`). Tmux's session env inheritance is subtle across versions; explicit `-e` on every create is the safest.
- Existing sessions are untouched. If `tmux::session_exists` is true, twrk only attaches — no new env vars are pushed into the live session. (Out of scope; user can kill + reopen if needed.)

---

## File Structure

- `src/tmux.rs` — `build_commands` signature gains an `env: &[(&str, &str)]` parameter; `-e KEY=VALUE` pairs are appended to `new-session`, `new-window`, and `split-window` argv (before the trailing shell-command). Tests updated to pass `&[]` for existing cases, new tests added for env vars.
- `src/main.rs` — computes the env vec from `active_group` and `worktree_name`, passes it to `build_commands`.
- `README.md` — new "Session env vars" subsection between `## Project config` and the next section.

No new files. No new modules.

---

## Task 1: Add `env` parameter to `build_commands` and inject `-e` flags (TDD)

**Files:**
- Modify: `src/tmux.rs:9-90` (the `build_commands` body)
- Modify: `src/tmux.rs:184-247` (existing tests — update `build_commands` call sites)
- Test: `src/tmux.rs` (inline `#[cfg(test)] mod tests` — add new env-var tests)

- [ ] **Step 1: Update existing tests to pass `&[]` for env**

The current tests call `build_commands("s", &PathBuf::from("/tmp"), &layout)`. After we change the signature to `build_commands(session, cwd, layout, env)`, those calls must pass `&[]` for env.

In `src/tmux.rs`, in the `tests` module, replace every call to `build_commands(...)` with the same call plus a trailing `&[]`. There are six call sites in `first_window_uses_new_session`, `second_pane_in_first_window_uses_horizontal_split`, `rows_window_uses_vertical_split`, `second_window_uses_new_window`, `unnamed_window_gets_default_name`, `final_select_window_targets_first`, and `pane_title_set_when_named`. Example:

```rust
let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
```

- [ ] **Step 2: Add new failing tests for env injection**

Append to the `tests` module in `src/tmux.rs`:

```rust
#[test]
fn env_vars_are_added_to_new_session() {
    let env = [("TWRK_CONFIG", "dev"), ("TWRK_WORKTREE", "1")];
    let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
    let ns = &cmds[0];
    assert_eq!(ns[0], "new-session");
    let pairs: Vec<&String> = ns
        .windows(2)
        .filter(|w| w[0] == "-e")
        .map(|w| &w[1])
        .collect();
    assert!(pairs.iter().any(|p| p.as_str() == "TWRK_CONFIG=dev"));
    assert!(pairs.iter().any(|p| p.as_str() == "TWRK_WORKTREE=1"));
}

#[test]
fn env_vars_are_added_to_new_window() {
    let env = [("TWRK_CONFIG", "dev")];
    let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
    let nw = cmds.iter().find(|c| c[0] == "new-window").unwrap();
    let pairs: Vec<&String> = nw
        .windows(2)
        .filter(|w| w[0] == "-e")
        .map(|w| &w[1])
        .collect();
    assert!(pairs.iter().any(|p| p.as_str() == "TWRK_CONFIG=dev"));
}

#[test]
fn env_vars_are_added_to_split_window() {
    let env = [("TWRK_CONFIG", "dev")];
    let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
    let sw = cmds.iter().find(|c| c[0] == "split-window").unwrap();
    let pairs: Vec<&String> = sw
        .windows(2)
        .filter(|w| w[0] == "-e")
        .map(|w| &w[1])
        .collect();
    assert!(pairs.iter().any(|p| p.as_str() == "TWRK_CONFIG=dev"));
}

#[test]
fn env_var_precedes_trailing_shell_command() {
    let env = [("TWRK_CONFIG", "dev")];
    let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &env);
    let ns = &cmds[0];
    let last = ns.last().unwrap();
    assert_eq!(last, "claude");
    let last_e = ns.iter().rposition(|s| s == "-e").unwrap();
    assert!(last_e < ns.len() - 2);
}

#[test]
fn no_env_means_no_dash_e_flags() {
    let cmds = build_commands("s", &PathBuf::from("/tmp"), &layout_two_windows(), &[]);
    for cmd in &cmds {
        assert!(
            !cmd.iter().any(|s| s == "-e"),
            "unexpected -e in {cmd:?}"
        );
    }
}
```

- [ ] **Step 3: Run the tests to verify the new ones fail and the old ones don't compile**

Run:

```bash
cargo test 2>&1 | tail -30
```

Expected: compile error on `build_commands` signature mismatch (extra arg `&env`/`&[]`). That's fine — it confirms the signature needs updating. If you renamed the call sites correctly in Step 1, you'll instead see five new test failures referencing missing env behaviour, plus the existing tests passing.

- [ ] **Step 4: Update `build_commands` signature and inject `-e` flags**

Replace the function body in `src/tmux.rs` (currently lines 8-85). The new signature takes a fourth parameter `env: &[(&str, &str)]`. After every `new-session`, `new-window`, and `split-window` push, append `-e KEY=VALUE` pairs to that argv **before** (or, since the shell-command is appended later in the same builder, you can interleave — see the order below).

Important ordering: each create command is built as a `Vec<String>` and pushed in one piece. Within that vec, the order is:

1. Subcommand (`new-session` / `new-window` / `split-window`)
2. All flags (`-d`, `-s`, `-n`, `-c`, `-h`/`-v`, etc.)
3. All `-e KEY=VALUE` pairs
4. Trailing shell-command (if any)

Concretely, in `build_commands`, replace lines 8-85 with:

```rust
#[must_use]
pub fn build_commands(
    session: &str,
    cwd: &Path,
    layout: &Layout,
    env: &[(&str, &str)],
) -> Vec<Vec<String>> {
    let mut cmds: Vec<Vec<String>> = Vec::new();
    let cwd_s = cwd.display().to_string();
    let layout = ensure_named_windows(layout);

    for (idx, window) in layout.iter().enumerate() {
        let win_name = window.name.as_deref().unwrap();
        let first_pane = window.content.first().map(|e| {
            let Element::Pane(p) = e;
            p
        });
        let first_cmd = first_pane
            .map(|p| p.command.as_str())
            .filter(|c| !c.is_empty());

        let mut create = if idx == 0 {
            vec![
                "new-session".into(),
                "-d".into(),
                "-s".into(),
                session.into(),
                "-n".into(),
                win_name.into(),
                "-c".into(),
                cwd_s.clone(),
            ]
        } else {
            vec![
                "new-window".into(),
                "-t".into(),
                format!("{session}:"),
                "-n".into(),
                win_name.into(),
                "-c".into(),
                cwd_s.clone(),
            ]
        };
        push_env(&mut create, env);
        if let Some(cmd) = first_cmd {
            create.push(cmd.into());
        }
        cmds.push(create);

        if let Some(pane) = first_pane
            && let Some(name) = &pane.name
        {
            cmds.push(vec![
                "select-pane".into(),
                "-t".into(),
                format!("{session}:{win_name}"),
                "-T".into(),
                name.clone(),
            ]);
        }

        for element in window.content.iter().skip(1) {
            let Element::Pane(pane) = element;
            let mut split = vec![
                "split-window".into(),
                split_flag(window.split).into(),
                "-t".into(),
                format!("{session}:{win_name}"),
                "-c".into(),
                cwd_s.clone(),
            ];
            push_env(&mut split, env);
            if !pane.command.is_empty() {
                split.push(pane.command.clone());
            }
            cmds.push(split);
            if let Some(name) = &pane.name {
                cmds.push(vec![
                    "select-pane".into(),
                    "-t".into(),
                    format!("{session}:{win_name}"),
                    "-T".into(),
                    name.clone(),
                ]);
            }
        }

        cmds.push(vec![
            "select-pane".into(),
            "-t".into(),
            format!("{session}:{win_name}.{{top-left}}"),
        ]);
    }

    if let Some(first) = layout.first() {
        cmds.push(vec![
            "select-window".into(),
            "-t".into(),
            format!("{session}:{}", first.name.as_deref().unwrap()),
        ]);
    }

    cmds
}
```

Then add the private helper at the bottom of `src/tmux.rs` (just above the `#[cfg(test)]` block), next to `split_flag` and `ensure_named_windows`:

```rust
fn push_env(args: &mut Vec<String>, env: &[(&str, &str)]) {
    for (key, value) in env {
        args.push("-e".into());
        args.push(format!("{key}={value}"));
    }
}
```

- [ ] **Step 5: Run all tests to verify they pass**

Run:

```bash
cargo test 2>&1 | tail -15
```

Expected: all tests pass (the original seven plus the five new env-var tests = 42 total).

- [ ] **Step 6: Commit**

```bash
git add src/tmux.rs
git commit -m "feat: Thread env vars through tmux::build_commands

- Add an \`env: &[(&str, &str)]\` parameter to \`build_commands\`
- Emit \`-e KEY=VALUE\` pairs on every \`new-session\`, \`new-window\`, and \`split-window\` before the trailing shell-command
- Add \`push_env\` private helper next to other tmux argv builders
- Cover env injection with unit tests on each create command"
```

---

## Task 2: Compute env vars in `main.rs` and pass them to `build_commands`

**Files:**
- Modify: `src/main.rs:119-123` (the `if !tmux::session_exists` block)

- [ ] **Step 1: Build the env vec from existing state**

In `src/main.rs`, replace lines 119-123 (currently):

```rust
    if !tmux::session_exists(&session_name) {
        let layout = config::resolve_layout(&cfg, &active_group);
        let cmds = tmux::build_commands(&session_name, &session_cwd, &layout);
        tmux::run(&cmds)?;
    }
```

with:

```rust
    if !tmux::session_exists(&session_name) {
        let layout = config::resolve_layout(&cfg, &active_group);
        let mut env: Vec<(&str, &str)> = vec![("TWRK_CONFIG", active_group.as_str())];
        if worktree_name.is_some() {
            env.push(("TWRK_WORKTREE", "1"));
        }
        let cmds = tmux::build_commands(&session_name, &session_cwd, &layout, &env);
        tmux::run(&cmds)?;
    }
```

- [ ] **Step 2: Build to verify the code compiles**

Run:

```bash
cargo build 2>&1 | tail -5
```

Expected: `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` with no errors.

- [ ] **Step 3: Run all tests**

Run:

```bash
cargo test 2>&1 | tail -10
```

Expected: all 42 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: Inject TWRK_CONFIG and TWRK_WORKTREE into tmux sessions

- Build env vec from \`active_group\` and \`worktree_name\` in \`real_main\`
- Always set \`TWRK_CONFIG\` (resolved config group name, e.g. \`default\`)
- Set \`TWRK_WORKTREE=1\` only when the session opened in a worktree
- Pass env vec into \`build_commands\` so each pane inherits it"
```

---

## Task 3: Manual integration check

**Files:**
- No code changes — runtime verification only.

- [ ] **Step 1: Install the local build**

Run:

```bash
cargo install --path . 2>&1 | tail -5
```

Expected: `Installed package \`twrk v1.1.0\``.

- [ ] **Step 2: Open a session against the twrk repo with a non-default config (if available) and verify env vars**

Pick any project with a `.twrk.*` config that has a non-`default` group, or use this repo if it has one. From outside tmux:

```bash
twrk /Users/paulvandermeijs/Workspace/twrk -c default
```

(Adjust the path / config name to a project you have. If you don't have a non-default group anywhere, `-c default` is fine — `TWRK_CONFIG` should still be set to `default`.)

Once attached, in any pane that has a shell (or after the pane's command exits if it isn't a shell), run:

```bash
env | grep TWRK
```

Expected output (worktree off):

```
TWRK_CONFIG=default
```

Expected output (if you ran with `-w`):

```
TWRK_CONFIG=default
TWRK_WORKTREE=1
```

If `TWRK_CONFIG` is missing, the `-e` flag isn't reaching panes — check tmux version (`tmux -V`) and inspect `tmux show-environment -t <session>`.

- [ ] **Step 3: Verify the round-trip use case**

Inside the session, run:

```bash
twrk . --worktree --config="$TWRK_CONFIG"
```

Expected: a new session opens for a fresh worktree of the current project, using the same config group as the parent session. Detach (`prefix d`) and confirm both sessions exist with `tmux ls`.

- [ ] **Step 4: Kill the test sessions**

```bash
tmux kill-session -t <name>
```

(Repeat for the worktree session. Clean up any worktrees with `git worktree remove` from the main checkout.)

No commit — this task is verification only.

---

## Task 4: Document env vars in README

**Files:**
- Modify: `README.md` (insert a new subsection after the existing `## Project config` block, before whatever follows)

- [ ] **Step 1: Find the insertion point**

Run:

```bash
grep -n "^## " README.md
```

Identify the section header that follows `## Project config`. Insert the new subsection immediately before it.

- [ ] **Step 2: Add the new subsection**

Insert this block in `README.md` directly after the `## Project config` section ends (after its example block):

```markdown
## Session env vars

Every tmux session twrk creates has these variables set, available to every pane process:

- `TWRK_CONFIG` — the resolved config group name (e.g. `default`, `dev`).
- `TWRK_WORKTREE` — `1` when the session opened in a git worktree, unset otherwise.

This lets you re-invoke twrk from inside an existing session and re-use the same config — handy for spinning up a worktree session that mirrors the parent:

    twrk . --worktree --config="$TWRK_CONFIG"
```

- [ ] **Step 3: Verify the file renders sensibly**

Run:

```bash
grep -n "Session env vars" README.md
```

Expected: exactly one match, in the correct ordinal position relative to other `## ` headings.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: Document TWRK_CONFIG and TWRK_WORKTREE session env vars"
```

---

## Out of scope

- Updating env vars on an **existing** session (attach path). Sessions are created once; if a user wants to switch config, they kill and reopen. Adding `set-environment -t <session>` on attach would need separate consideration around overwriting state.
- Per-pane env overrides (e.g. one pane gets a different `TWRK_CONFIG`). Not requested.
- Exposing the worktree **name** as `TWRK_WORKTREE` (vs. the literal `1`). Could be a follow-up if the literal `1` proves limiting in practice.
- Forwarding arbitrary user-defined env vars from the config file. Distinct feature; revisit if requested.
