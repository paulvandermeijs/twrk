# Always-on cliclack Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Always frame twrk's output in a cliclack `intro`/`outro` ribbon. When a project path is provided to the CLI (direct mode), replace the skipped interactive prompts with cliclack `log::step` lines that announce the chosen options.

**Architecture:** `main()` becomes the single owner of the cliclack ribbon — it installs the theme, emits the `intro` at startup, and routes both success and failure to `outro` / `outro_cancel`. The picker-mode guard around `intro`/`outro` is removed. `prompts.rs` gains three thin display helpers (`show_project`, `show_config`, `show_worktree`) used only in direct mode, in the spots where the corresponding prompt is skipped. No new module is introduced; `prompts.rs` is the natural home since it already wraps cliclack interactions.

**Tech Stack:** Rust 2024, cliclack 0.5

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `src/main.rs` | CLI entry + orchestration | Move theme + intro to top of `main()`; route every error path through `outro_cancel`; emit `outro` unconditionally; call new `show_*` helpers in direct-mode branches |
| `src/prompts.rs` | All cliclack-facing UI (interactive + non-interactive) | Add `show_project`, `show_config`, `show_worktree` |

No new files. No tests added — the new helpers are thin pass-throughs to `cliclack::log::step` which writes to stderr; there is no logic to unit-test. Visual verification in Task 4 covers behavior.

---

## Task 1: Always frame `main()` output with cliclack

**Files:**
- Modify: `src/main.rs:18-32` (the `main` function) and `src/main.rs:34-43` and `src/main.rs:98-100` (intro + outro placement inside `real_main`)

- [ ] **Step 1: Inspect current `main()` and intro/outro placement**

Run: `sed -n '18,44p;98,108p' src/main.rs`

Expected: confirms `theme::install()` and `cliclack::intro(...)` live inside `real_main` (only on the picker branch), `cliclack::outro` lives at line ~99 inside `if in_picker_mode`, and the error branch in `main` uses `eprintln!` for direct mode.

- [ ] **Step 2: Hoist theme install + intro into `main()`**

Replace `src/main.rs:18-32` with:

```rust
fn main() -> ExitCode {
    let args = cli::Args::parse();
    theme::install();
    let _ = cliclack::intro(console::style(" twrk ").bold().black().on_color256(213));
    let in_picker_mode = args.path.is_none();
    match real_main(args, in_picker_mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let _ = cliclack::outro_cancel(format!("{e:#}"));
            ExitCode::FAILURE
        }
    }
}
```

Notes:
- `eprintln!("twrk: {e:#}")` for direct-mode errors is removed — `outro_cancel` replaces it now that the ribbon is always open.
- `theme::install` is idempotent (sets a global), safe to run on every invocation.

- [ ] **Step 3: Remove the now-duplicated theme + intro inside `real_main`**

In `src/main.rs`, the `else` branch of the `args.path` `if let` currently looks like:

```rust
} else {
    theme::install();
    let _ = cliclack::intro(console::style(" twrk ").bold().black().on_color256(213));
    let roots = workspace::roots()?;
    let projects = workspace::list_projects(&roots);
    picker::pick(&projects)?
};
```

Replace with:

```rust
} else {
    let roots = workspace::roots()?;
    let projects = workspace::list_projects(&roots);
    picker::pick(&projects)?
};
```

(`theme::install()` and the `intro(...)` call are now done unconditionally in `main()`.)

- [ ] **Step 4: Drop the picker-mode guard around the success outro**

In `src/main.rs`, the `if in_picker_mode` block before `tmux::session_exists` currently looks like:

```rust
if in_picker_mode {
    let _ = cliclack::outro(format!("Launching {session_name}..."));
}
```

Replace with:

```rust
let _ = cliclack::outro(format!("Launching {session_name}..."));
```

- [ ] **Step 5: Build to confirm everything compiles**

Run: `cargo build`

Expected: `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` with no errors. Warnings about unused `in_picker_mode` are fine for now (we still use it for prompt branching in Task 3).

- [ ] **Step 6: Run existing tests**

Run: `cargo test --quiet`

Expected: all 37 tests pass (no behavioral logic changed yet — only output framing).

- [ ] **Step 7: Smoke-test direct mode to confirm errors now surface as outro_cancel**

Run: `cargo run --quiet -- /tmp/definitely-not-a-real-twrk-path-xyz` (or any non-existent path)

Expected: cliclack ribbon opens, then closes with `outro_cancel` styling (red bar) and the error message. No bare `eprintln!` line.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs
git commit -m "refactor: Always frame output with cliclack intro/outro

- Move \`theme::install\` and \`cliclack::intro\` to the top of \`main\`
  so the ribbon opens for every invocation (not just picker mode)
- Route every error through \`outro_cancel\`; drop the bare \`eprintln!\`
  used for direct-mode errors
- Emit the success \`outro\` unconditionally"
```

---

## Task 2: Add display helpers to `prompts.rs`

**Files:**
- Modify: `src/prompts.rs` (append three new public functions; add `Path` import)

- [ ] **Step 1: Read the current top of `src/prompts.rs` to confirm imports**

Run: `sed -n '1,5p' src/prompts.rs`

Expected: `use anyhow::{Context, Result};` is the only import.

- [ ] **Step 2: Add `Path` import and the three display helpers**

At the top of `src/prompts.rs`, add a `use std::path::Path;` line below the `anyhow` import:

```rust
use std::path::Path;

use anyhow::{Context, Result};
```

Then, immediately above the `#[must_use]` line for `resolve_group_name` (i.e. before line 35), insert:

```rust
pub fn show_project(path: &Path) {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
    let _ = cliclack::log::step(format!("Project: {name}"));
}

pub fn show_config(group: &str) {
    let _ = cliclack::log::step(format!("Config: {group}"));
}

pub fn show_worktree(name: Option<&str>) {
    let label = match name {
        Some(n) => format!("Worktree: {n}"),
        None => "Worktree: none".to_string(),
    };
    let _ = cliclack::log::step(label);
}
```

Notes:
- These mirror the `let _ = cliclack::intro(...)` ignore-error pattern used elsewhere — failing to write a log line shouldn't crash the run.
- `log::step` uses the theme's submit symbol/colour, so the visual matches a completed prompt (which is exactly the framing: "this option was decided").
- `show_project` displays only the folder name (the same identifier used for the tmux session) to match what the picker shows in interactive mode.

- [ ] **Step 3: Confirm the unused-import lint doesn't fire on `Context` in this file**

Run: `cargo build`

Expected: clean build; `Context` and `Result` are still used by the existing prompt functions.

- [ ] **Step 4: Commit**

```bash
git add src/prompts.rs
git commit -m "feat: Add cliclack log helpers for chosen options

- Add \`show_project\`, \`show_config\`, \`show_worktree\` to \`prompts.rs\`
- Each prints a single \`cliclack::log::step\` line so the visual
  matches a completed prompt (used in direct mode where the
  corresponding prompt is skipped)"
```

---

## Task 3: Call the display helpers in direct mode

**Files:**
- Modify: `src/main.rs` (`real_main`)

- [ ] **Step 1: Show the project after path resolution (direct mode only)**

In `src/main.rs`, the `if let Some(p) = args.path.as_deref()` branch currently looks like:

```rust
let project_dir = if let Some(p) = args.path.as_deref() {
    path::resolve(p)?
} else {
    let roots = workspace::roots()?;
    let projects = workspace::list_projects(&roots);
    picker::pick(&projects)?
};
```

Replace with:

```rust
let project_dir = if let Some(p) = args.path.as_deref() {
    let resolved = path::resolve(p)?;
    prompts::show_project(&resolved);
    resolved
} else {
    let roots = workspace::roots()?;
    let projects = workspace::list_projects(&roots);
    picker::pick(&projects)?
};
```

- [ ] **Step 2: Show the config group after it's resolved (direct mode only)**

Immediately after the line:

```rust
let active_group =
    prompts::resolve_group_name(args.config.as_deref(), picked_group.as_deref());
```

add:

```rust
if !in_picker_mode {
    prompts::show_config(&active_group);
}
```

- [ ] **Step 3: Show the worktree decision after the worktree branch resolves (direct mode only)**

The block that picks `(session_cwd, worktree_name, folder_source)` currently ends like this in `src/main.rs`:

```rust
let (session_cwd, worktree_name, folder_source): (PathBuf, Option<String>, PathBuf) =
    if want_worktree && let Some(root) = git::repo_root(&project_dir) {
        let name = name_override.unwrap_or_else(session::random_name);
        let path = git::ensure_worktree(&root, &name)?;
        (path, Some(name), root)
    } else {
        (project_dir.clone(), None, project_dir.clone())
    };
```

Immediately after that `let` (on the line right after the closing `};`), add:

```rust
if !in_picker_mode {
    prompts::show_worktree(worktree_name.as_deref());
}
```

- [ ] **Step 4: Build**

Run: `cargo build`

Expected: clean build.

- [ ] **Step 5: Run tests**

Run: `cargo test --quiet`

Expected: all 37 tests pass (no logic changed in the tested helpers).

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: Echo chosen options as log lines in direct mode

- Call \`show_project\` after resolving \`--path\`
- Call \`show_config\` after the active config group is resolved
- Call \`show_worktree\` after the worktree decision is made
- All three only fire when a path was provided on the CLI
  (picker mode already shows these via the prompts themselves)"
```

---

## Task 4: Visual verification

**Files:** none — purely runtime checks.

The cliclack helpers can't be meaningfully unit-tested (output goes to stderr via global theme state). Verify each scenario by eye.

- [ ] **Step 1: Direct mode, all flags explicit**

Run: `cargo run --quiet -- . -c default -w smoke-test`

Expected output (approximate):

```
◆  twrk
│
◇  Project: twrk
│
◇  Config: default
│
◇  Worktree: smoke-test
│
└  Launching twrk-smoke-test...
```

Detach immediately (`Ctrl-b d` from inside tmux) and clean up the worktree:

```bash
git worktree remove .worktrees/smoke-test
git branch -d smoke-test
tmux kill-session -t twrk-smoke-test 2>/dev/null
```

- [ ] **Step 2: Direct mode, no flags (config/worktree fall through to defaults)**

Run: `cargo run --quiet -- .`

Expected: ribbon shows `Project`, `Config: default`, `Worktree: none`, then `Launching twrk...` outro. No prompts.

Detach and clean up:

```bash
tmux kill-session -t twrk 2>/dev/null
```

- [ ] **Step 3: Picker mode (no path) — confirm prompts still appear and no duplicate echoes**

Run: `cargo run --quiet`

Expected: project picker → config picker (only if multiple groups) → worktree confirm → outro. The new `show_*` helpers must NOT fire (picker mode already surfaces these via the prompts themselves). If duplicate `Project: …` lines appear after the project picker, the `if !in_picker_mode` guards in Task 3 are wrong — fix and re-verify.

Press `Ctrl-c` to cancel; expected: red `outro_cancel` ribbon with the cancellation error.

- [ ] **Step 4: Direct mode, error path (non-existent path)**

Run: `cargo run --quiet -- /tmp/nope-not-real`

Expected: ribbon opens, then closes via `outro_cancel` (red bar) with the underlying error chain. No bare `eprintln!` line above or below the ribbon.

- [ ] **Step 5: Final commit if any nits were fixed during verification**

If you needed to tweak anything during Steps 1–4, commit those tweaks now:

```bash
git add -p
git commit -m "polish: <describe the visual nit you fixed>"
```

If no fixes were needed, skip this step.

---

## Self-Review Notes

- **Spec coverage:** "always output using cliclack" → Task 1. "use cliclack log functions to output chosen options" → Task 2 + Task 3. "finish with the outro" → Task 1, Step 4 (outro now unconditional).
- **Direct mode definition:** "project is provided to the CLI" = `args.path.is_some()` = `!in_picker_mode`. Used consistently in the guards.
- **Type consistency:** `show_project(&Path)`, `show_config(&str)`, `show_worktree(Option<&str>)` — all called with values that exist at the call sites (`PathBuf::as_path` via `&` deref, `String`, `Option<String>::as_deref`).
- **No placeholders.** All steps include exact code or exact commands.
