# Sheprd Flok Operator Contract

Use this repository skill when the current project is operated through the
`m-mohamed.sheprd` Herdr plugin.

## Invariants

- Herdr owns sessions, workspaces, panes, persistence, remotes, agent state,
  keybindings, and live IDs.
- Sheprd owns project resolution, the explicit 2x2 Flok, model defaults,
  isolated worker worktrees, state receipts, and safe cleanup.
- A Flok contains exactly four visible agents: Pi conducts; Codex, Claude Code,
  and OpenCode work.
- Pi must not edit project files. Never add hidden subagents or a fifth coding
  agent.
- Treat repository checks and worktree state as evidence; do not infer success
  from an agent's prose.

## Start

```bash
herdr plugin action invoke m-mohamed.sheprd.doctor
herdr plugin action invoke m-mohamed.sheprd.open-flok
# or
herdr plugin action invoke m-mohamed.sheprd.choose-flok
```

Then resolve the live roster instead of guessing names or IDs:

```bash
herdr agent list
```

`open-flok` focuses an existing workspace without reshaping it. Inspect the
returned `healthy` field and `warnings`; a focused workspace is not necessarily
a healthy four-agent roster.

## Conduct

Pi should send bounded, self-contained packets to the three visible workers:

```bash
herdr agent prompt <agent-name> '<task with scope, checks, and stop conditions>' --wait --timeout 120000
herdr agent read <agent-name> --source recent-unwrapped --lines 120 --format text
```

Treat `herdr --skill` from the installed binary as the command authority.
Sheprd intentionally supports Herdr 0.7.5 and newer and is verified against
Herdr 0.8.0; do not confuse the compatibility floor with the current runtime.
Do not copy older `agent send`, `agent wait --status`, or implicit agent-start
examples. Outside a Herdr-managed pane, do not control a live session.

Workers own separate branches and worktrees. Before synthesis, inspect the
actual diff, commit, test output, and working-tree state for each worker. Pi may
coordinate integration but must not silently edit the base checkout.

## Factory run

Use the deterministic factory when the task has explicit paths and checks:

```bash
target/release/sheprd factory run <project> --task '<bounded task>' \
  --allow-path <repo-relative-path> --check '<check command>' --json
```

The Rust runner owns phase order and parses one fresh nonce-bound, sentinel-delimited
typed JSON envelope per agent turn. Pi plans; Codex implements and receives no more than
two check-driven correction turns; Claude reviews intent; OpenCode reviews
adversarially. Rust runs checks without an agent, using `/bin/sh -c`, a bounded
timeout, and source-state mutation detection. Agent-owned ignored mutations are
rejected; check-owned ignored outputs are excluded from the reviewed patch.
Acceptance is false unless the checks pass, both reviews approve, the base
checkout and worker HEAD remain unchanged, and every actual changed path is
allowed. Never treat a rejected receipt as an integration result; inspect the
preserved worker checkout.

## Cleanup

Preview through the headless action:

```bash
herdr plugin action invoke m-mohamed.sheprd.cleanup-preview
```

Use the interactive action for mutation:

```bash
herdr plugin action invoke m-mohamed.sheprd.cleanup-flok
```

The overlay requires the active project name. Cleanup refuses dirty or
out-of-scope paths, preserves branches, and archives the state receipt. Never
delete a worker checkout manually until its Git state and branch are understood.

## Diagnose

From a linked source checkout:

```bash
target/release/sheprd doctor --json
target/release/sheprd flok <project> --json
target/release/sheprd cleanup <project> --json
herdr plugin log list --plugin m-mohamed.sheprd
```

JSON launch output includes the current workspace, panes, agent names, models,
effort, branches, worktree paths, health, warnings, and state path. Herdr IDs
are session-local and must not be stored as durable configuration.

Read [docs/commands.md](docs/commands.md) for the binary contract and
[agent-guide.md](agent-guide.md) for human onboarding and troubleshooting.
