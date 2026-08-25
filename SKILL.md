# Sheprd project-router contract

Read the [agent guide](agent-guide.md) and [command reference](docs/commands.md) for the operator-facing teaching surface.

Use Sheprd when a task needs canonical project resolution, Herdr workspace
focus, the editor-first sample recipe, or readiness checks. Use the Ratatui
factory cockpit for command-and-control and HQ's Sol/Luna launcher for bounded
parallel work.

## Boundary

- Herdr owns live runtime state and pane/workspace IDs.
- Sheprd owns project discovery, canonical checkouts, focus, recipes, and
  readiness.
- Tuxedo owns private task truth.
- Pi owns orchestration policy.
- Git and deterministic checks own code evidence.

Retired peer-agent integrations and the old receipt-backed fleet CLI are not
supported. Do not invoke or recreate them.

## Daily commands

```bash
factory
factory --json
sheprd list --json
sheprd connect <project> --recipe agent-dev
sheprd doctor --json
sheprd show-config
```

Use the focused workspace for ordinary tasks. Open the Sol/Luna runbook from
the cockpit when a task genuinely benefits from scout, builder, and verifier
parallelism.

## Sol/Luna launch contract

Launch only inside a Herdr-managed pane and provide:

- canonical project;
- Tuxedo task ID and line number;
- one bounded outcome;
- explicit repository-relative allow paths;
- deterministic checks;
- a stop condition and human acceptance gate.

```bash
~/workspace/hq/workflows/sol-luna-launch.sh \
  --project <project> \
  --task-id <id> \
  --task-number <line> \
  --task "<bounded outcome>" \
  --allow-path <path> \
  --check "<check>"
```

The live fleet is one Pi Sol-Hi conductor plus three Codex Luna-Max workers.
There are no hidden agents. OpenCode Go DeepSeek V4 Flash at `max` is an
explicit review path, not a fourth worker.

## Evidence

Inspect worker branches, changed paths, checks, and the private receipt. Agent
prose is not completion. Finalize with:

```bash
~/workspace/hq/workflows/sol-luna-finalize.sh \
  --receipt ~/.local/state/sol-luna/<project>/<run>/receipt.json
```

Never reset unrelated dirty WIP or merge/push without human acceptance.
