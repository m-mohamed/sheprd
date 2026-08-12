# Sheprd Agent Guide

This guide is for an agent helping a human install, operate, or troubleshoot
Sheprd. Sheprd is a Herdr plugin; Flok is its opinionated four-agent workflow.

## Explain the boundary first

Herdr is the terminal runtime. It owns live workspaces, tabs, panes, agent
state, persistence, remotes, keybindings, attach/detach, and the socket API.
Sheprd asks Herdr to create or focus one known layout and never stores live IDs
as configuration.

The Flok grid is:

```text
Pi · conductor       │ Codex · GPT-5.6 Sol
─────────────────────┼──────────────────────
Claude · Opus 5      │ OpenCode · DeepSeek V4 Flash
```

All four default to high effort. Pi conducts from the clean base checkout with
read/search/shell tools and no direct editing tools. The three workers receive
separate branches and worktrees. There are no hidden subagents.

OpenCode retains its native permission prompts. If it blocks on access outside
its worker checkout, Pi should surface the request and wait for a human rather
than bypassing it.

## Install and preflight

```bash
herdr plugin install m-mohamed/sheprd
herdr plugin action list --plugin m-mohamed.sheprd
herdr plugin action invoke m-mohamed.sheprd.doctor
```

The installer fetches an exact-version release archive, checks SHA-256 and
GitHub provenance, and falls back to a locked source build only when a verified
prebuilt is unavailable. Herdr does not sandbox plugins; tell the human to
review the manifest and named scripts.

Doctor verifies Herdr, Git, Pi, Codex, Claude Code, and OpenCode. It does not
prove that provider billing, model access, or credits are available.

## Open and inspect

```bash
herdr plugin action invoke m-mohamed.sheprd.open-flok
herdr plugin action invoke m-mohamed.sheprd.choose-flok
herdr agent list
```

A new Flok refuses a dirty base checkout. A matching existing Flok is focused,
never silently repaired. If structured output says `healthy: false`, explain
the warnings and inspect the live roster; do not equate focus with readiness.

Use Herdr's normal focus and zoom controls. Sheprd does not replace its UI.

## Coordinate work

Pi should issue explicit packets with a purpose, owned files, expected checks,
done criteria, and stop conditions. Use current Herdr commands:

```bash
herdr agent prompt <name> '<self-contained packet>'
herdr agent wait <name> --until idle --timeout 120000
herdr agent read <name> --source recent --lines 120 --format text
```

Before reporting completion, inspect repository state and test receipts. Agent
messages are claims, not merge proof.

For an explicit receipt-backed run, Pi first creates a typed plan. Then use
`sheprd factory run` with `--plan-file`, a bounded task, repository-relative
`--allow-path` values, and `--check` commands. The command preserves rejected
work and returns trace and receipt paths. It never integrates the worker branch.

## Clean up safely

```bash
herdr plugin action invoke m-mohamed.sheprd.cleanup-preview
herdr plugin action invoke m-mohamed.sheprd.cleanup-flok
```

The second action opens an overlay and requires the project name. Sheprd checks
that every path belongs to its state root, refuses dirty worktrees, closes the
workspace first, removes clean checkouts, preserves branches, and archives the
state JSON. If anything becomes dirty during shutdown, it stops and preserves
the checkout.

## Troubleshoot by layer

1. **Plugin registration**

   ```bash
   herdr plugin list
   herdr plugin action list --plugin m-mohamed.sheprd
   herdr plugin log list --plugin m-mohamed.sheprd
   ```

2. **Runtime readiness**

   ```bash
   herdr status server
   target/release/sheprd doctor --json
   ```

3. **Project resolution**

   ```bash
   target/release/sheprd show-config --json
   target/release/sheprd list --json
   ```

4. **Flok state and live roster**

   ```bash
   target/release/sheprd flok <project> --json
   herdr workspace list
   herdr pane list
   herdr agent list
   ```

The structured Flok result contains workspace and pane IDs, agent names,
models, branches, worktree paths, state path, health, and warnings. Herdr IDs
are live values. Do not guess or persist them in dotfiles.

Sheprd uses `HERDR_BIN_PATH` and Herdr CLI wrappers. Raw socket code is only
appropriate for a future long-lived event subscriber or custom client that the
CLI cannot express.
