# Sheprd agent guide

Sheprd is a thin Herdr project router. The Ratatui factory cockpit is the
operator command center; Sheprd is not a multiplexer, swarm scheduler, or
receipt-backed fleet runtime.

## Start here

```bash
factory --json
sheprd list --json
sheprd connect <project> --recipe agent-dev
sheprd doctor --json
```

Use `sheprd connect` to focus or create the small editor-first workspace. It
must not reshape a live workspace implicitly. Herdr owns workspace, tab, pane,
and agent IDs; these Herdr IDs are ephemeral, so read them from Herdr responses whenever needed.

## Active fleet

For bounded parallel work, run HQ's launcher inside a Herdr pane:

```bash
~/workspace/hq/workflows/sol-luna-launch.sh \
  --project <project> \
  --task-id <tuxedo-project-id> \
  --task-number <line> \
  --task "<bounded outcome>" \
  --allow-path <repo-relative-path> \
  --check "<deterministic check>"
```

The topology is finite and visible:

```text
Sol-Hi / Pi conductor
├── Luna 1 / scout   — read-only inventory and risks
├── Luna 2 / builder — implementation in declared paths
└── Luna 3 / verifier — independent checks and corrections
```

Models are `gpt-5.6-sol` with high thinking for Sol and `gpt-5.6-luna` with
xhigh reasoning for Luna. OpenCode Go uses DeepSeek V4 Flash at `variant: max`
when a separate review is useful. No hidden agents are allowed.

## Acceptance

A run is not complete because a pane is idle. Inspect actual branches, changed
paths, check output, and the private receipt. Finalize evidence before cleanup:

```bash
~/workspace/hq/workflows/sol-luna-finalize.sh \
  --receipt ~/.local/state/sol-luna/<project>/<run>/receipt.json
```

Then accept, request one correction, defer, or reject. Never merge or push
without human approval. Preserve dirty unrelated work.

Retired peer-agent and legacy fleet integrations must not be documented,
invoked, or restored.
