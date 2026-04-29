# Picker-flow prompts: config and worktree

## Goal

Extend the interactive picker flow so that, after selecting a project, the user can also select which config group to use, whether to create a git worktree, and what to name it. Today the equivalent decisions are only reachable via the `-c` and `-w` CLI flags.

## Scope

- These prompts run **only in picker mode** — that is, when `twrk` is invoked with no path argument.
- When the user passes a path (`twrk .`, `twrk ~/Projects/Site`), behavior is unchanged. The path-mode flow stays the fast, flag-driven path.

## Flow (picker mode)

1. **Project picker** — unchanged.
2. **Load config** for the selected project — unchanged.
3. **Config picker** — new. Shown when:
   - `-c` was **not** passed on the command line, **and**
   - The merged config contains 2 or more groups.

   The items are the group names from the config (the keys of the `BTreeMap`). The user's choice becomes the active group name used everywhere `args.config` is currently read.

4. **Worktree confirm** — new. Shown when:
   - The selected project is a git repo (`git::repo_root` returns `Some`), **and**
   - `-w` was **not** passed on the command line.

   The toggle defaults to the active group's `worktree` value, falling back to `false` if the group is missing or has no `worktree` field.

5. **Worktree name input** — new. Shown only when step 4 resolves to yes.
   - A random name is generated up front via `session::random_name`.
   - The random name is shown as the prompt placeholder.
   - If the user submits an empty value, the random name is used.
   - If the user types a value, that value is used.

When any of the new prompts is skipped, the value used is whatever the existing logic in `main.rs` already produces for that decision today.

## CLI changes

- `cli::Args::config` becomes `Option<String>` with no clap default.
  - When `Some(name)`, behavior is identical to today's `-c name`.
  - When `None`, picker mode runs the config prompt (if multi-group); path mode and single-group picker mode fall back to the literal string `"default"`.
- `cli::Args::worktree` is unchanged — it already distinguishes `None` (not passed) from `Some("")` (passed with no value) from `Some(name)`.

## Code organisation

- A new module `prompts.rs` holds the two new interactive prompts:
  - `pick_config(group_names: &[String]) -> Result<String>`
  - `pick_worktree(default: bool, placeholder_name: &str) -> Result<Option<String>>` — returns `None` if the user picks "no worktree", `Some(name)` if yes (where `name` is either the typed value or the placeholder when the user submitted empty).

  These functions wrap `cliclack::select` / `cliclack::confirm` / `cliclack::input` calls. They are not unit-tested directly because `cliclack` interactive prompts don't unit-test cleanly.

- `main.rs` orchestrates the new prompts. The existing block that resolves `(want_worktree, name_override)` from `args.worktree` is extended:
  - When `args.path` is `None` (picker mode) and the corresponding flag is also `None`, call into `prompts.rs` to get the answer.
  - Otherwise, fall through to the current logic.

- `picker.rs` is not touched. It stays focused on project selection.

- A small pure helper `resolve_group_name(flag: Option<&str>, picked: Option<&str>) -> String` lives in `prompts.rs` (or `main.rs`) and is unit-tested.

## Edge cases

- **No config file at all.** `config::load_for` returns an empty map. The config prompt is skipped (zero groups < 2). The active group name is `"default"`, which falls through to the existing fallback layout. The worktree confirm still runs if the project is a git repo, defaulting to `false`.
- **Config has exactly one group.** Config prompt is skipped; that single group becomes the active group regardless of its name.
- **Active group's `worktree` is unset.** Toggle default is `false`.
- **User submits empty worktree name.** Random name (the placeholder) is used.
- **Project is not a git repo.** Worktree prompts are skipped entirely; behavior matches today.
- **Both `-c` and `-w` passed in picker mode.** Both prompts are skipped; flow is identical to today's behavior with those flags.

## Tests

- `cli.rs` tests are updated for `Option<String>` config — the existing assertions on `args.config` change from `"default"` to `None` when the flag isn't passed.
- New unit tests for the pure `resolve_group_name` helper covering: flag set, flag unset + picker answered, flag unset + no picker answer.
- No new tests for the prompt wrappers themselves.

## Out of scope

- README updates — covered separately.
- Persisting last-chosen group/worktree across runs.
- Allowing the user to back out of the worktree-name prompt to the toggle. Cliclack cancel still exits the program, same as elsewhere in the tool.
