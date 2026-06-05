# sheprd

`sheprd` is a smart session manager for Herdr.

Herdr owns the terminal runtime: sessions, workspaces, tabs, panes, persistence,
agent state, remotes, keybindings, integrations, and attach/detach.

`sheprd` owns the human entry point: find the project, choose the agent, apply a
small workspace recipe, and connect to the matching Herdr workspace.

```text
tmux  : sesh
Herdr : sheprd
```

## Install

`sheprd` is source-first while the project is young:

```bash
cargo build --release
install -m 755 target/release/sheprd ~/.local/bin/sheprd
```

## Commands

```bash
sheprd list
sheprd connect my-project
sheprd open my-project
sheprd switch my-project
sheprd recipes
sheprd doctor
sheprd show-config
```

`connect`, `open`, and `switch` currently share behavior: create or focus the
Herdr workspace for a project. The names are all present because the product is
a session manager, not just a launcher.

## Default Recipe

The first built-in recipe is `agent-dev`:

- `code`: `nvim`, selected agent, shell
- `git`: `lazygit`, shell

The recipe is intentionally small. Herdr remains the place for manual pane work,
session navigation, remotes, and agent state.

## Config

Create `~/.config/sheprd/config.toml` when defaults are not enough:

```toml
roots = [
  "~/Workspace",
  "~/code",
  "~/src",
]

ignore = [
  ".git",
  ".direnv",
  ".tmp",
  "node_modules",
  "target",
  "vendor",
]

max_depth = 6
default_agent = "codex"
```

Supported agents are `pi`, `droid`, `claude`, `codex`, `hermes`, and
`opencode`.

## Herdr Contract

`sheprd` uses Herdr's documented CLI wrappers:

- `herdr workspace list/create/focus`
- `herdr tab create/rename/focus`
- `herdr pane split/rename/run/report-agent`
- `herdr status`

It does not store Herdr workspace, tab, or pane IDs. IDs are live runtime state,
so `sheprd` reads them from Herdr command responses each time.

## Development

```bash
just ci
```

Without `just`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```
