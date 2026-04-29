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
twrk                          # pick a project; default config group
twrk .                        # use current dir, default config group
twrk -c dev                   # "dev" config group
twrk -w                       # create a worktree with a random name
twrk -w my-feature            # create a worktree named "my-feature"
twrk ~/Projects/Site          # path outside workspace, no worktree
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

Happy twrk-ing!
