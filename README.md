# sheprd

<p align="center">
  <img src="website/assets/sheprd-mark.svg" alt="sheprd" width="88" />
</p>

<p align="center">
  <a href="#install">install</a> ·
  <a href="#quick-start">quick start</a> ·
  <a href="#configuration">configuration</a> ·
  <a href="#herdr-contract">Herdr contract</a> ·
  <a href="#development">development</a>
</p>

---

**A small, sharp entry layer for Herdr workspaces.**

Herdr owns the terminal runtime: sessions, workspaces, tabs, panes,
persistence, agent state, remotes, keybindings, integrations, and attach/detach.

`sheprd` owns the way in: find the project, choose the agent lane, and connect
to the matching Herdr workspace. Optional sample recipes can shape a fresh
workspace when you ask for them.

```text
tmux  : sesh
Herdr : sheprd
```

`sheprd` should earn trust by being boring, legible, and precise about what it
does not own.

## What You Get

- **Project discovery.** Scan configured roots and explicit project mappings for
  Git repositories.
- **Agent lanes.** Open the same project for `codex`, `opencode`, `droid`,
  `pi`, `hermes`, or `claude` without changing Herdr itself.
- **Workspace focus.** Create or focus the matching Herdr workspace without
  memorizing live workspace IDs.
- **Doctor checks.** Verify Herdr, required tools, and Herdr server
  protocol/socket readiness before you blame the wrong layer. JSON output
  exposes typed Herdr runtime fields for agents and scripts.
- **Sample recipes.** Apply an explicit starter layout only when creating a
  fresh workspace.

## How It Compares

| | Herdr | sheprd |
| --- | --- | --- |
| Runtime sessions | owns | uses |
| Workspaces, tabs, panes | owns | creates/focuses through Herdr |
| Persistence and attach/detach | owns | does not own |
| Keybindings and UI | owns | does not own |
| Project discovery | not its job | owns |
| Agent lane naming | supports agents | selects the requested lane |
| Sample starter layouts | can be driven by CLI/API | opt-in recipes |

If a feature would turn `sheprd` into a terminal multiplexer, layout engine, or
Herdr replacement, it belongs in a discussion before code.

## Install

`sheprd` is source-first while the project is young:

```bash
git clone https://github.com/m-mohamed/sheprd
cd sheprd
scripts/install-local.sh
```

Set `SHEPRD_INSTALL_DIR` to install somewhere other than `~/.local/bin`:

```bash
SHEPRD_INSTALL_DIR=/usr/local/bin scripts/install-local.sh
```

Herdr must be installed separately. Start with the Herdr install docs:
<https://herdr.dev/docs/install/>.

## Quick Start

Preview the default config:

```bash
sheprd init --print
```

Write a starter config:

```bash
sheprd init
```

Check the environment:

```bash
sheprd doctor
```

List projects:

```bash
sheprd list
```

Connect to a project:

```bash
sheprd connect my-project
```

`connect`, `open`, and `switch` share the same baseline behavior: create or
focus the Herdr workspace for a project. They do not force panes, tabs, or
commands by default.

Use a starter layout only when you want one:

```bash
sheprd connect my-project --recipe agent-dev
```

`agent-dev` creates a fresh workspace with:

- `code`: `nvim`, selected agent, shell
- `git`: `lazygit`, shell

Recipes are intentionally small and optional. Re-running `connect` for an
existing workspace focuses it instead of reshaping live panes.

## Commands

```bash
sheprd init --print
sheprd init
sheprd list
sheprd connect my-project
sheprd connect my-project --recipe agent-dev
sheprd open my-project
sheprd switch my-project
sheprd recipes
sheprd doctor
sheprd show-config
```

Use `--agent` to pick a lane:

```bash
sheprd --agent opencode connect my-project
sheprd --agent droid list
```

Use `--json` when scripts or agents need stable output:

```bash
sheprd list --json
sheprd doctor --json
sheprd init --print --json
sheprd connect my-project --json
sheprd recipes --json
sheprd show-config --json
```

`sheprd doctor --json` includes a typed `herdr` block with server running state,
version, protocol, compatibility, socket path, and `protocol_ready`. Agents
should use that field instead of scraping human check details.

`sheprd connect --json` reports the resolved project, selected agent, Herdr
workspace label/id, whether it focused or created, recipe use, and whether an
interactive Herdr client was attached. JSON mode does not launch an interactive
Herdr client; add a separate `sheprd connect my-project` when you want the human
terminal attached.

If you are already inside Herdr, use `--no-attach` to avoid nesting clients:

```bash
sheprd connect my-project --no-attach
```

## Configuration

Create `~/.config/sheprd/config.toml` when defaults are not enough:

```bash
sheprd init --print
sheprd init --root ~/Workspace --root ~/src
```

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

Supported agents:

- `pi`
- `droid`
- `claude`
- `codex`
- `hermes`
- `opencode`

## Herdr Contract

`sheprd` uses Herdr's documented CLI wrappers:

- `herdr workspace list/create/focus`
- `herdr tab create/rename/focus`
- `herdr pane split/rename/run/report-agent`
- `herdr status`

It does not store Herdr workspace, tab, or pane IDs. IDs are live runtime state,
so `sheprd` reads them from Herdr command responses each time.

Herdr's native runtime API makes future non-TUI clients possible. `sheprd`
should stay a small companion: prefer CLI wrappers while they cover project
connection, and add raw socket code only for protocol-client features such as
dashboards, mobile clients, or event subscriptions. Any socket path must first
check Herdr status, protocol compatibility, and socket location.

## Docs

- [Product foundation](docs/product-foundation.md)
- [Herdr precedent](docs/herdr-precedent.md)
- [Open-source readiness](docs/open-source-readiness.md)
- [Prelaunch chaos checklist](docs/prelaunch-chaos.md)
- [Release process](docs/release.md)
- [`SKILL.md`](SKILL.md): agent-facing usage contract

## Agent Instructions

If you are an AI agent helping in this repository, read
[`AGENTS.md`](AGENTS.md) before making changes and read
[`CONTRIBUTING.md`](CONTRIBUTING.md) before opening issues or PRs.

## Development

Install repo-local hooks once:

```bash
just install-hooks
```

Run the normal checks:

```bash
just ci
```

Run the fuller prelaunch gate:

```bash
just check
```

Without `just`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Before a public release, run the prelaunch checklist in
[`docs/prelaunch-chaos.md`](docs/prelaunch-chaos.md). Do not tag, publish, or
flip repository visibility until the explicit final ship gate.

## License

`sheprd` is licensed under AGPL-3.0-or-later.
