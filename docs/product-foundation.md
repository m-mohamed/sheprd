# Product Foundation

Sheprd keeps exactly four coding agents in frame. Flok is the opinionated mode;
Herdr is the runtime.

## Ownership

Herdr owns:

- sessions, workspaces, tabs, panes, focus, zoom, and persistence;
- remote access, attach/detach, keybindings, and agent lifecycle state;
- plugin installation, invocation context, logs, and the CLI/socket protocol.

Sheprd owns:

- project discovery and explicit project names;
- one documented 2x2 Pi/Codex/Claude/OpenCode layout;
- model and effort defaults;
- clean-check protection and isolated worker worktrees;
- operation locking, state receipts, health comparison, rollback, and cleanup.
- the deterministic factory phase machine, typed envelopes, code-run checks, scope validation, traces, and acceptance receipts.

If a feature makes Sheprd a multiplexer, general layout engine, session store,
keybinding system, remote layer, or hidden agent harness, it belongs elsewhere.

## Flok contract

Pi conducts from the clean base checkout without direct edit tools. Codex,
Claude Code, and OpenCode each work in a dedicated branch/worktree. The roster
is visible and fixed at four.

A new Flok is transactional: prerequisites and cleanliness precede mutation;
live roster verification precedes the atomic state receipt. Partial failures
remove only resources still proven clean. Existing Floks are focused and
inspected, never implicitly repaired.

Cleanup is equally explicit: preview first, typed confirmation or `--confirm`,
path ownership and cleanliness checks, workspace close, clean checkout removal,
branch preservation, and state archival.

Factory runs separate orchestration from execution safety. Pi supplies a typed
plan. Codex implements, Rust checks, and Claude/OpenCode review. Sheprd decides
acceptance from scope, checks, reviews, and repository state. The runner never
integrates or silently cleans rejected work.

## Product surfaces

The primary surface is the Herdr manifest:

```bash
herdr plugin action invoke m-mohamed.sheprd.open-flok
herdr plugin action invoke m-mohamed.sheprd.choose-flok
herdr plugin action invoke m-mohamed.sheprd.cleanup-flok
```

The binary exists for deterministic testing, JSON automation, and recovery:

```bash
sheprd doctor --json
sheprd flok <project> --json
sheprd cleanup <project> --json
```

The older `connect` and sample-recipe commands remain compatibility surfaces;
they are no longer the product's main story.

## Honest limits

Doctor can prove paths and Herdr protocol readiness. It cannot pre-authorize
models or prove provider credits, billing, rate limits, or future model-name
stability. A worker's output is not merge proof; repository and test evidence
remain required.
