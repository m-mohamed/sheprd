# Sheprd

Sheprd is the small Herdr project router. It discovers canonical Git projects,
opens or focuses a workspace, applies the optional editor-first recipe, and
reports readiness. It does not own command-and-control or launch a hidden
agent fleet.

## Product boundary

- **Ratatui factory cockpit:** primary command-and-control surface.
- **Tuxedo:** private task truth and done signals.
- **Sheprd:** project resolution, workspace focus, recipes, and readiness.
- **Herdr:** live panes, tabs, sessions, focus, and agent state.
- **HQ Sol/Luna workflows:** explicit bounded parallel work and private receipts.

Retired peer-agent integrations and the legacy fleet CLI are not part of the
supported command, plugin, or configuration surface.

[Docs index](docs/README.md) · [Command reference](docs/commands.md) · [`agent-guide.md`](agent-guide.md)

## Install

```bash
cargo install --locked --path .
# or use the managed Herdr plugin installer
herdr plugin install m-mohamed/sheprd
```

## Commands

```bash
sheprd init --print
sheprd list --json
sheprd connect <project> --recipe agent-dev
sheprd recipes
sheprd doctor --json
sheprd show-config
```

`connect` only creates or focuses a workspace. It never repairs an existing
workspace implicitly and never starts an agent fleet. The `agent-dev` recipe is
a small editor, selected-agent, shell, and lazygit sample layout.

## Ratatui command center

Use the cockpit for the daily loop:

```bash
factory
factory --json
```

The cockpit renders Tuxedo tasks, Herdr health, subscription usage, project
focus, the Sol/Luna runbook, and Obsidian. It stores no authority state.

## Sol/Luna workflow

For a task that benefits from independent scout, builder, and verifier roles,
run from a Herdr-managed pane:

```bash
~/workspace/hq/workflows/sol-luna-launch.sh \
  --project <name-or-path> \
  --task-id <tuxedo-project-id> \
  --task-number <line> \
  --task "<bounded outcome>" \
  --allow-path <repo-relative-path> \
  --check "<deterministic check>"
```

The launcher creates one Sol-Hi Pi conductor and three visible Codex Luna-Max
worktrees. It refuses a dirty base checkout, requires explicit scope and
checks, and writes a private receipt under `~/.local/state/sol-luna`.

Finalize evidence before previewing cleanup:

```bash
~/workspace/hq/workflows/sol-luna-finalize.sh \
  --receipt ~/.local/state/sol-luna/<project>/<run>/receipt.json
~/workspace/hq/workflows/sol-luna-cleanup.sh \
  --receipt ~/.local/state/sol-luna/<project>/<run>/receipt.json
```

OpenCode Go is the deliberate review path with
`opencode-go/deepseek-v4-flash` and `variant: max`. Claude remains optional
legacy tooling and is not required by Sheprd.

## Development

```bash
just check
just chaos-smoke
just failure-smoke
```

The CLI emits JSON for automation and fail-closed errors on `--json`. Herdr
owns live IDs; do not store or guess them. Dirty repository WIP is preserved.
