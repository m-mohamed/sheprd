# Sheprd Agent Guide

Use this guide when an agent is helping a human understand, set up, or
troubleshoot `sheprd`.

This guide is different from `SKILL.md`:

- `SKILL.md` is for an agent operating a repository that already uses Sheprd.
- `agent-guide.md` is for an agent teaching a human what Sheprd is and how to
  use it safely.

## Core Model

Sheprd is a Herdr companion, not a terminal runtime.

- Herdr owns sessions, workspaces, tabs, panes, persistence, remotes,
  keybindings, integrations, attach/detach, and agent state.
- Sheprd owns project discovery, project selection, agent-lane selection,
  readiness checks, config bootstrap, optional sample recipes, and
  automation-friendly outcomes.

The short version:

```text
tmux  : sesh
Herdr : sheprd
```

## First Conversation With A Human

Start with the boundary before commands:

1. Sheprd helps you enter the right Herdr workspace.
2. Plain `connect` creates or focuses a workspace.
3. Recipes are optional samples, not the default worldview.
4. JSON mode is for agents and scripts.
5. Herdr ids are live runtime ids; do not store or guess them.

Then run the smallest checks:

```bash
sheprd doctor
sheprd list
```

If the machine is new or the config is unclear:

```bash
sheprd init --print
sheprd show-config
```

## Setup Path

Install Herdr first. Sheprd assumes Herdr is the runtime.

Then install Sheprd from source while the project is young:

```bash
git clone https://github.com/m-mohamed/sheprd
cd sheprd
scripts/install-local.sh
```

If the user wants a different install directory:

```bash
SHEPRD_INSTALL_DIR=/usr/local/bin scripts/install-local.sh
```

Do not present source install as a polished package-manager release until the
public launch gate approves that path.

## Daily Use

The normal loop is:

```bash
sheprd list
sheprd connect my-project
```

If the user is already inside Herdr or an automation is running:

```bash
sheprd connect my-project --no-attach
```

For scripts and coding agents:

```bash
sheprd doctor --json
sheprd list --json
sheprd connect my-project --json
```

When `--json` is used, read structured fields instead of scraping human text.
Runtime failures after argument parsing emit a JSON error envelope on stderr:

```json
{
  "ok": false,
  "error": {
    "kind": "message",
    "message": "project 'missing' was not found",
    "exit_code": 2
  }
}
```

## Recipes

Teach recipes as explicit samples.

```bash
sheprd connect my-project --recipe agent-dev
```

The `agent-dev` sample creates a fresh workspace with:

- `code`: `nvim`, selected agent, shell
- `git`: `lazygit`, shell

If the workspace already exists, Sheprd focuses it and does not reshape live
tabs or panes.

## Troubleshooting

Use `doctor --json` when the human says Herdr, agents, or sockets feel wrong:

```bash
sheprd doctor --json
```

Read these fields:

- `ready`
- `herdr.running`
- `herdr.protocol`
- `herdr.compatible`
- `herdr.socket`
- `herdr.protocol_ready`
- `checks`

Common interpretations:

- `herdr.protocol_ready: false`: Herdr is missing, stopped, incompatible, or has
  no usable socket.
- missing selected agent executable: the agent lane is configured, but the tool
  is not on `PATH`.
- missing `nvim` or `lazygit`: the optional `agent-dev` sample will not work
  cleanly until those tools exist.
- project not found: run `sheprd list` and verify the configured roots.
- path is not a git repository: pass the repository root, not an arbitrary
  directory.

## What Not To Do

Do not teach Sheprd as:

- a Herdr replacement;
- a terminal multiplexer;
- a pane layout engine;
- a keybinding layer;
- a remote/SSH layer;
- a task database;
- a personal operating system.

Do not tell agents or users to store Herdr workspace, tab, or pane ids as
durable config.

Do not make `agent-dev` sound like required policy. It is a sample recipe.

## Source Trail

For exact behavior, use:

- `README.md` for the human entry point.
- `docs/commands.md` for command contracts.
- `SKILL.md` for an agent operating Sheprd in a repo.
- `AGENTS.md` for maintainers changing the repository.
- `docs/prelaunch-chaos.md` for final prelaunch proof.
