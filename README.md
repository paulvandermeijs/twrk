# twrk

Open a project from your workspace in a tmux session with a configurable layout, optionally in a fresh git worktree.

## Install

```bash
cargo install --path .
```

## Usage

```
twrk                                    # pick a project; default config group
twrk .                                  # use current dir, default config group
twrk -c dev -n my-feature              # named session, "dev" config group
twrk ~/Projects/Site --worktree=false  # path outside workspace, no worktree
twrk -x "ls -la"                       # run a command in the project, no tmux
```

## Workspace roots

Set `WORKSPACE_ROOT` (newline-separated, `~` allowed). Defaults to `~/Workspace`.

## Project config

Place `.twrk.toml`, `.twrk.yaml`, or `.twrk.json` in the project (or any parent directory). Lower directories override higher ones.

The top level is a map of named config groups. Pick one with `-c <name>` (default `default`). Each group can set `worktree` and a `layout` (a list of windows, each with `content`).

```yaml
default:
  worktree: false
  layout:
    - name: Project
      content:
        - { name: Shell, command: nu }
dev:
  worktree: true
  layout:
    - name: Dev
      content:
        - { name: Claude, command: claude -r }
        - { name: Editor, command: hx }
    - name: Server
      content:
        - { command: npm run dev }
    - name: Logs
      split: rows
      content:
        - { command: "tail -f /var/log/app.log" }
```
