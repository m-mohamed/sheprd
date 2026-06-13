# sheprd

`sheprd` is a smart session manager for Herdr.

Herdr owns the terminal runtime: sessions, workspaces, tabs, panes, persistence,
agent state, remotes, keybindings, integrations, and attach/detach.

`sheprd` owns the human entry point: find the project, choose the agent lane,
and connect to the matching Herdr workspace. Optional sample recipes can shape a
fresh workspace when you ask for them.

```text
tmux  : sesh
Herdr : sheprd
```

## Install

`sheprd` is source-first while the project is young:

```bash
scripts/install-local.sh
```

Set `SHEPRD_INSTALL_DIR` to install somewhere other than `~/.local/bin`.

## Commands

```bash
sheprd list
sheprd connect my-project
sheprd connect my-project --recipe agent-dev
sheprd open my-project
sheprd switch my-project
sheprd recipes
sheprd doctor
sheprd show-config
```

`connect`, `open`, and `switch` share the same baseline behavior: create or
focus the Herdr workspace for a project. They do not force panes, tabs, or
commands by default.

## Sample Recipes

The first bundled sample recipe is `agent-dev`:

- `code`: `nvim`, selected agent, shell
- `git`: `lazygit`, shell

Use it when you want `sheprd` to shape a fresh workspace as a starter layout:

```bash
sheprd connect my-project --recipe agent-dev
```

Sample recipes are intentionally small and optional. Herdr remains the place for
manual pane work, session navigation, remotes, and agent state. This keeps
`sheprd` closer to a session selector like `sesh`, not a personal layout that
everyone has to accept.

## Config

Create `~/.config/sheprd/config.toml` when defaults are not enough:

```toml
roots = [
  "~/Workspace",
  "~/code",
  "~/src",
]

[[projects]]
name = "my-project"
path = "~/workspace/startups/my-project-main-worktree"

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

Use `[[projects]]` entries when a project name should point at a specific path
that would otherwise discover under the wrong directory name. Explicit projects
keep the configured `name` while still using the configured `path` as the Herdr
workspace cwd.

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

Before a public release, run the prelaunch checklist in
[`docs/prelaunch-chaos.md`](docs/prelaunch-chaos.md). The open-source readiness
scorecard lives in [`docs/open-source-readiness.md`](docs/open-source-readiness.md),
and the publish checklist lives in [`docs/public-launch.md`](docs/public-launch.md).
