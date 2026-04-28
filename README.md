# twrk

Open a project from your workspace in a tmux session with a configurable layout, optionally in a fresh git worktree.

## Install

```bash
cargo install --path .
```

## Usage

```
twrk                                    # pick a project; default layout
twrk .                                  # use current dir, default layout
twrk -l dev -n my-feature              # named session, "dev" layout
twrk ~/Projects/Site --worktree=false  # path outside workspace, no worktree
twrk -x "ls -la"                       # run a command in the project, no tmux
```

## Workspace roots

Set `TWRK_WORKSPACE` (newline-separated, `~` allowed). Defaults to `~/Workspace`.

## Project config

Place `.twrk.toml`, `.twrk.yaml`, or `.twrk.json` in the project (or any parent directory). Lower directories override higher ones.

```yaml
worktree: true
layout:
  default:
    - name: Project
      content:
        - { name: Shell, command: nu }
  dev:
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
