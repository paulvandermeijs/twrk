# twrk

Pronounced "twerk". The name is a portmanteau of tmux and workspace/worktree — any resemblance to other words is, of course, purely coincidental.

Open a project from your workspace in a tmux session with a configurable layout, optionally in a fresh git worktree.

`twrk` builds on [`wrk`](https://github.com/paulvandermeijs/wrk).

## Install

```bash
cargo install twrk
```

Or from a local checkout:

```bash
cargo install --path .
```

## Usage

```
twrk                          # interactive: pick project, config group, worktree
twrk .                        # use current dir, skip the picker
twrk ~/Projects/Site          # use any absolute path (outside the workspace too)
twrk -c dev                   # use the "dev" config group
twrk -w                       # create a worktree with a random name
twrk -w my-feature            # create a worktree named "my-feature"
```

## Workspace roots

Set `WORKSPACE_ROOT` (newline-separated, `~` allowed). Defaults to `~/Workspace`.

## Project config

Place a twrk-file (`.twrk.toml`, `.twrk.yaml`, or `.twrk.json`) in the project (or any parent directory). Lower directories override higher ones.

The top level of a twrk-file is a map of named config groups. Pick one with `-c <name>` (default `default`). Each group can set `worktree` and a `layout` (a list of windows, each with `content`).

```yaml
default:
  worktree: false
  layout:
    - name: project
      content:
        - { name: shell, command: nu }
dev:
  worktree: true
  layout:
    - name: dev
      content:
        - { name: claude, command: claude -r }
        - { name: editor, command: hx }
    - name: server
      content:
        - { command: npm run dev }
    - name: logs
      split: rows
      content:
        - { command: "tail -f /var/log/app.log" }
```

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

- `TWRK_CONFIG` — the resolved config group name (e.g. `default`, `dev`).
- `TWRK_WORKTREE` — always `1` (setup only runs when a worktree is involved).
- `TWRK_WORKTREE_NAME` — the worktree name (as passed to `-w`, or the auto-generated random one).
- `TWRK_REPO_ROOT` — absolute path to the source repo, useful when `..` isn't enough.

## Session env vars

Every tmux session twrk creates has these variables set, available to every pane process:

- `TWRK_CONFIG` — the resolved config group name (e.g. `default`, `dev`).
- `TWRK_WORKTREE` — `1` when the session opened in a git worktree, unset otherwise.

This lets you re-invoke twrk from inside an existing session and re-use the same config — handy for spinning up a worktree session that mirrors the parent:

```bash
twrk . --worktree --config="$TWRK_CONFIG"
```

Happy twrk-ing!
